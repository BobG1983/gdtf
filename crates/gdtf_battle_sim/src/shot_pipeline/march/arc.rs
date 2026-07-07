//! The **arc (lobbed) march** — [`march_arc`] flies one LOBBED grenade along a
//! deterministic parabola from the thrower's cell to a target cell and reports where it
//! lands (GTW-546, child GTW-41d of GTW-41).
//!
//! This is the [`TrajectoryStyle::Arc`](crate::weapon::TrajectoryStyle) counterpart to the
//! flat [`march_vector`](super::march_vector). Unlike a straight round, a lobbed grenade
//! RISES over same-level cover and DESCENDS onto the target cell — so it is NOT stopped by
//! walls / cover / occupants along the way (it arcs above them). Its ONLY blocker is an
//! intact ROOF: as the parabola crosses a storey boundary the floor/roof
//! [`SurfaceGrid`](crate::surface::SurfaceGrid) slab is tested (one slab, both faces — the
//! SAME z-boundary check the straight march's `advance_z` uses):
//! [`SlabState::Present`](crate::surface::SlabState) STOPS the lob at that boundary (an
//! intact roof — the grenade cannot pass), while
//! [`SlabState::Destroyed`](crate::surface::SlabState) / `Absent` PASSES (a hole / window /
//! open sky — the grenade drops through). Reaching the target cell's column LANDS the
//! grenade there.
//!
//! ## Determinism (GTW-546 risk note)
//!
//! The parabola is a PURE function of `(thrower, target)` — NO RNG draw (unlike the
//! straight shot's cone sample). The apex height is a fixed function of horizontal distance,
//! and the parabola is sampled at a fixed sub-cell cadence, so two identical calls yield the
//! identical landing. Because a coarse XY sample could skip a thin roof slab the parabola
//! passes through, the walk tests EVERY storey boundary the sampled polyline crosses (not
//! just XY cells): each consecutive sample pair whose floored z differs triggers the slab
//! test at EACH plane the segment crosses — a steep segment can cross several storeys in
//! one step (GTW-645), so the planes are walked in flight order and the first intact slab
//! blocks — exactly mirroring the straight march's `advance_z` semantics.
//!
//! The LANDING [`MarchResult`] feeds the GTW-541 blast resolver
//! ([`aoe_affected`](crate::aoe::aoe_affected)) at its `at` cell — the throw dispatch fans
//! the weapon's [`HitType::Blast`](crate::weapon::HitType) there. Pure, render-free model
//! logic; **zero pixels**.

use crate::{
    clearance::round_band_for_cell,
    march::{
        dda::MAX_STEPS,
        geom::{key_of, key_of_clamped, xy_in_grid, z_in_grid},
        result::{MarchKind, MarchResult},
    },
    metric::{CellLevel, MAX_LEVELS, SimPos, cell_center, pos_to_cell},
    surface::{SlabState, SurfaceGrid},
    tuning::CombatTuning,
};

/// The horizontal sub-cell sampling cadence of the parabola, in sim units — a fixed, small
/// step so the polyline never skips a thin roof slab between two samples (GTW-546 risk
/// note: sample z-crossings explicitly). Four samples per cell (0.25 units) bounds the XY
/// travel per segment, but NOT the z travel: a STEEP lob (a multi-storey level change over
/// little horizontal run — the steepest being the same-column throw, whose whole parabola is
/// ONE segment) crosses several storey boundaries in a single step, so `roof_block_between`
/// walks EVERY crossed boundary rather than assuming one per segment (GTW-645).
const SAMPLE_STEP: f32 = 0.25;

/// The base apex height a lob rises ABOVE the straight muzzle→target line, in sim units
/// (storeys) — a deterministic constant so the parabola is a pure function of its endpoints.
/// HALF a storey: enough to clear same-level cover (which tops out below a storey) yet LOW
/// enough that a SAME-LEVEL lob never crosses into the storey above (`z + 0.5` still floors to
/// the same level), so a same-floor throw under an intact roof is NOT self-blocked — only a
/// throw that targets a DIFFERENT storey crosses (and is roof-gated at) the boundary between
/// them. A longer throw grows the apex ([`APEX_PER_CELL`]) but the growth is scaled to stay
/// sub-storey for realistic ranges.
const BASE_APEX: f32 = 0.5;

/// The per-cell apex growth, in storeys of extra rise per sim-unit of horizontal distance —
/// so a longer lob arcs marginally higher (a deterministic function of range, no RNG). Kept
/// tiny so a same-level throw's apex stays sub-storey across the grid's practical span (a
/// cross-map lob apexes ~half a storey higher, still not punching a same-level roof at short
/// range).
const APEX_PER_CELL: f32 = 0.02;

/// The parabolic z of the lob at horizontal fraction `u` in `0..=1` — a straight-line
/// interpolation from `z0` (muzzle) to `z1` (target) PLUS a symmetric parabolic bump peaking
/// at `u == 0.5` with height `apex`. `4·u·(1−u)` is the unit parabola (0 at the ends, 1 at
/// the middle), scaled by `apex`, so the round rises then falls onto the target — the lob.
fn arc_z(z0: f32, z1: f32, u: f32, apex: f32) -> f32 {
    let straight = (z1 - z0).mul_add(u, z0);
    apex.mul_add(4.0 * u * (1.0 - u), straight)
}

/// March a LOBBED grenade along a deterministic parabola from `thrower` to `target` and
/// report where it lands (GTW-546) — the [`TrajectoryStyle::Arc`](crate::weapon::TrajectoryStyle)
/// counterpart to [`march_vector`](super::march_vector).
///
/// The parabola runs from the `thrower` cell centre to the `target` cell centre, rising to a
/// deterministic apex (a fixed function of horizontal distance) then descending. It is
/// sampled at a fixed sub-cell cadence (`SAMPLE_STEP`); the walk:
///
/// - **Passes** same-level cover / walls / occupants — a lob arcs OVER them, so they never
///   stop it (unlike the straight march). The grenade's only blocker is a roof.
/// - **Tests the roof** on every storey boundary the sampled polyline crosses — a steep
///   segment may cross several in one step; each is tested in flight order (GTW-645): an
///   intact [`SlabState::Present`](crate::surface::SlabState) slab STOPS the lob at that
///   boundary ([`MarchKind::Slab`]) — the grenade cannot pass an intact roof (AC: "arc
///   blocked by intact roofs"); a [`SlabState::Destroyed`](crate::surface::SlabState) /
///   `Absent` slab (a hole / window / open sky) PASSES (AC: "passes through holes and
///   windows").
/// - **Lands** at the `target` cell once the parabola settles there: the returned
///   [`MarchResult`] carries [`MarchKind::Ground`] at the target `(cell, level)` — the
///   LANDING cell the GTW-541 blast resolver fans its
///   [`HitType::Blast`](crate::weapon::HitType) from.
///
/// Blind-throw: this takes NO facing / line-of-sight input — the lob is resolved purely from
/// the two cells (`docs/combat/combat.md`: a lobbed grenade may be thrown blind). Degenerate
/// inputs (an out-of-grid thrower / target) return a graceful [`MarchKind::Miss`] at the
/// clamped start, never a panic. DETERMINISTIC: no RNG draw, so two identical calls yield the
/// identical landing (the seeded-replay property the blast fan relies on).
#[must_use]
pub fn march_arc(
    thrower: CellLevel,
    target: CellLevel,
    surface: &SurfaceGrid,
    tuning: &CombatTuning,
) -> MarchResult {
    let (thrower_cell, thrower_level) = thrower.split();
    let (target_cell, target_level) = target.split();
    let muzzle = cell_center(thrower_cell, thrower_level);
    let landing = cell_center(target_cell, target_level);

    // Degenerate / out-of-grid endpoints: a graceful Miss at the clamped thrower cell (AC
    // #7 no-panic parity with the straight march).
    if !xy_in_grid(thrower.x, thrower.y)
        || !z_in_grid(thrower.z)
        || !xy_in_grid(target.x, target.y)
        || !z_in_grid(target.z)
    {
        return MarchResult {
            kind:   MarchKind::Miss,
            at:     key_of_clamped(thrower.x, thrower.y, thrower.z),
            band:   round_band_for_cell(muzzle, tuning),
            impact: muzzle,
        };
    }

    let dx = landing.x - muzzle.x;
    let dy = landing.y - muzzle.y;
    let horizontal = dx.hypot(dy);
    // The lob's apex (storeys above the straight line) — a deterministic function of the
    // horizontal range. A zero-range self-throw still arcs the BASE_APEX and lands in place.
    let apex = horizontal.mul_add(APEX_PER_CELL, BASE_APEX);
    // The number of sub-cell samples along the parabola — at least one, capped by MAX_STEPS
    // so a pathological range can never spin (AC #7).
    let steps = arc_sample_count(horizontal);

    // Walk the sampled polyline. `prev` starts at the muzzle; each step advances the
    // horizontal fraction `u` and evaluates the parabola. On a storey-boundary crossing the
    // roof slab is tested; reaching the final sample (u == 1) lands at the target.
    let mut prev = muzzle;
    for i in 1..=steps {
        #[expect(
            clippy::cast_precision_loss,
            reason = "steps is a small sample count bounded by MAX_STEPS; the f32 fraction is exact for this range"
        )]
        let u = (i as f32) / (steps as f32);
        let point = SimPos::new(
            dx.mul_add(u, muzzle.x),
            dy.mul_add(u, muzzle.y),
            arc_z(muzzle.z, landing.z, u, apex),
        );
        // Test every storey boundary this segment crosses — an intact roof STOPS the lob
        // (AC: blocked by intact roofs), a hole / window PASSES (AC: passes through).
        if let Some(blocked) = roof_block_between(prev, point, surface, tuning) {
            return blocked;
        }
        prev = point;
    }

    // The parabola settled onto the target column with no roof intercept — the grenade lands
    // at the target cell. Reported as Ground (the impact surface the blast fans from), keyed
    // at the target (cell, level).
    MarchResult {
        kind:   MarchKind::Ground,
        at:     CellLevel::new(target_cell, target_level),
        band:   round_band_for_cell(landing, tuning),
        impact: landing,
    }
}

/// The number of parabola samples for a `horizontal` sim-unit range — the range divided by
/// the [`SAMPLE_STEP`] cadence, at least one, capped by [`MAX_STEPS`] so a pathological range
/// never spins (AC #7). A zero-range self-throw still samples once (it lands in place).
fn arc_sample_count(horizontal: f32) -> u32 {
    let raw = (horizontal / SAMPLE_STEP).ceil();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "raw is a non-negative ceil'd count clamped to 1.0..=MAX_STEPS below, so the u32 cast cannot wrap"
    )]
    let n = raw.clamp(1.0, f32::from(u16::MAX)) as u32;
    n.clamp(1, MAX_STEPS)
}

/// Test the roof slab on EVERY storey boundary the segment `prev → point` crosses — an
/// intact [`SlabState::Present`](crate::surface::SlabState) roof STOPS the lob at the FIRST
/// crossed boundary in flight order (returns [`Some`] with a [`MarchKind::Slab`] result at
/// that boundary), a [`SlabState::Destroyed`](crate::surface::SlabState) / `Absent` hole
/// PASSES that boundary (the walk continues to the next). Mirrors the straight march's
/// `advance_z` slab semantics: the slab between two storeys is keyed at the UPPER of the two
/// levels (the floor of the upper storey / roof of the lower).
///
/// A STEEP segment — a multi-storey z-change over one `SAMPLE_STEP` of horizontal travel;
/// the steepest is the same-column lob, whose whole parabola is ONE segment — can cross
/// SEVERAL storey planes in a single sample step (GTW-645: testing only `max(prev_z, cur_z)`
/// skipped every intermediate slab, letting a grenade sail through an intact roof). The walk
/// therefore visits each crossed plane in FLIGHT order — ascending while the lob rises,
/// descending while it falls — so the first intact roof the grenade meets is the one that
/// stops it, each tested where the segment crosses that plane ([`roof_block_at`]).
fn roof_block_between(
    prev: SimPos,
    point: SimPos,
    surface: &SurfaceGrid,
    tuning: &CombatTuning,
) -> Option<MarchResult> {
    let prev_z = floor_level(prev.z);
    let cur_z = floor_level(point.z);
    if prev_z == cur_z {
        return None; // no storey boundary crossed on this segment
    }
    // Every integer z-plane in `(min .. max]` separates two storeys this segment spans;
    // walk them in flight order so the FIRST intact roof met is the one that blocks.
    let low = prev_z.min(cur_z) + 1;
    let high = prev_z.max(cur_z);
    if cur_z > prev_z {
        // Rising: the lob meets the lowest crossed plane first.
        for boundary in low..=high {
            if let Some(blocked) = roof_block_at(prev, point, boundary, surface, tuning) {
                return Some(blocked);
            }
        }
    } else {
        // Falling: the lob meets the highest crossed plane first.
        for boundary in (low..=high).rev() {
            if let Some(blocked) = roof_block_at(prev, point, boundary, surface, tuning) {
                return Some(blocked);
            }
        }
    }
    None // every crossed slab is a hole / window — the lob passes through
}

/// Test ONE crossed storey plane `boundary` against the segment `prev → point` — [`Some`]
/// with the [`MarchKind::Slab`] block when the slab there is intact
/// ([`SlabState::Present`](crate::surface::SlabState)), [`None`] when the plane is out of
/// grid or the slab is a hole (`Destroyed` / `Absent`, AC: passes through holes / windows).
///
/// The XY cell tested is where the SEGMENT crosses the plane: the sampled polyline treats
/// each segment as a straight line, so the crossing is the linear interpolation of the
/// endpoints at `z == boundary` (clamped into the segment for float safety) — the roof the
/// grenade is actually under / over at that instant, not the segment-end cell (GTW-645: on
/// a steep segment the end cell can be a column the parabola only reaches a storey later).
/// The slab between two storeys is keyed at the UPPER of the two — `boundary` itself.
fn roof_block_at(
    prev: SimPos,
    point: SimPos,
    boundary: i32,
    surface: &SurfaceGrid,
    tuning: &CombatTuning,
) -> Option<MarchResult> {
    if !z_in_grid(boundary) {
        return None;
    }
    // In-grid boundaries are 0..MAX_LEVELS (checked above), so this conversion never fails —
    // taking the typed route (instead of an `as` cast) keeps the cast lints inert.
    let plane = f32::from(u8::try_from(boundary).ok()?);
    // Where the segment crosses the plane. The caller only calls with `floor(prev.z) !=
    // floor(point.z)`, so the segment's z-extent is non-zero and the division is well-formed;
    // the clamp guards the interpolant against float dust at the segment ends.
    let t = ((plane - prev.z) / (point.z - prev.z)).clamp(0.0, 1.0);
    let cross = SimPos::new(
        (point.x - prev.x).mul_add(t, prev.x),
        (point.y - prev.y).mul_add(t, prev.y),
        plane,
    );
    let (cell, _) = pos_to_cell(cross);
    let slab_key = key_of(cell.x, cell.y, boundary);
    if surface.slab_state(&slab_key) == SlabState::Present {
        // An intact roof stops the lob at this boundary (AC: blocked by intact roofs). The
        // grenade cannot pass — it lands against the roof where it crossed the plane.
        return Some(MarchResult {
            kind:   MarchKind::Slab,
            at:     slab_key,
            band:   round_band_for_cell(cross, tuning),
            impact: cross,
        });
    }
    None // a Destroyed / Absent slab is a hole / window — the lob passes this plane
}

/// Floor a sim-unit `z` to its storey index, clamped into the representable
/// `0..=MAX_LEVELS` range so a sub-floor / above-ceiling sample never wraps the cast.
fn floor_level(z: f32) -> i32 {
    let floored = z.floor();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to the i32 range below, so the cast cannot wrap; the fractional part is gone after floor"
    )]
    let clamped = floored.clamp(i32::MIN as f32, i32::MAX as f32) as i32;
    clamped.clamp(0, i32::from(MAX_LEVELS))
}
