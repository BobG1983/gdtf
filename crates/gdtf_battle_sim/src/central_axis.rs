//! The §1 **central axis** — the muzzle point, the target aim point, and the
//! recoil-climb-tilted firing axis, all in **sim units / level-fractions**
//! (`docs/combat/resolution.md` §1 "muzzle/aim-point paragraph" + "What's pure
//! math vs sim"; `docs/combat/battle-space.md` §"Stance / cover / muzzle / aim
//! heights" + §"Sub-cell precision on the ground plane").
//!
//! Aiming centres a dispersion cone whose central axis runs from the **3D muzzle
//! point** to the **aim point** (resolution.md §1). This module derives those two
//! points and the recoil-climb tilt of the axis between them — the geometry the
//! cone sample (E2.5) and the coarse march (§2) build on. Everything here is in the
//! cubic-voxel metric ([`crate::metric::SimPos`], one cell = one level = 1.0 sim
//! unit): vertical datums are **level-fractions**, sub-cell offsets are
//! **cell-fractions**, and "up" is `+z` ([`Vec3::Z`]). **Zero pixels.**
//!
//! Three derivations, each reading its magnitudes from tuning / E1 data — nothing
//! hardcoded:
//!
//! 1. [`muzzle_position`] — the shooter's [`cell_center`] plus the per-facing
//!    forward offset ([`crate::tuning::MuzzleForwardOffset`], a cell-fraction along
//!    [`Direction::forward_step`]), **clamped** so the muzzle never leaves the
//!    shooter's cell; `z = level as f32 + the per-stance muzzle level-fraction`
//!    ([`crate::tuning::MuzzleHeights`]).
//! 2. [`target_aim_point`] — the target's [`cell_center`] with a z derived per-case:
//!    a **ganger** target aims at its per-stance silhouette-top level-fraction
//!    (the dedicated [`crate::tuning::SilhouetteTops`] tuning) ×
//!    [`crate::tuning::AimHeightFrac`]; a **cover-occupied** cell aims at the cover
//!    [`HeightBand`]'s midpoint level-fraction (derived from the [`ProjectileBandEdges`]
//!    band edges, never a literal — cover height genuinely IS band-based).
//! 3. [`climb_aim_dir`] — the unit muzzle→aim axis, tilted **up** (`+z`) by
//!    `prior_shots × recoil_climb × recoil_growth` radians (resolution.md §1a recoil
//!    climb); zero prior shots yields the untilted axis exactly. Returns the named
//!    unit-direction newtype [`AimDir`].

use bevy::math::Vec3;

use crate::{
    cone::PriorShots,
    cover::HeightBand,
    ganger::{Facing, Position, Stance, StanceKind},
    metric::{Cell, Level, SimPos, cell_center},
    stability::RecoilGrowth,
    tuning::{
        AimHeightFrac, CombatTuning, MuzzleHeight, ProjectileBandEdges, RecoilClimb, SilhouetteTop,
    },
};

/// A **unit firing-axis direction** in the cubic-voxel metric — the (possibly
/// recoil-tilted) muzzle→aim axis the cone is sampled about.
///
/// A named newtype over [`Vec3`] (no-bare-types: a firing direction is a domain
/// value, not a bare vector) whose invariant is **unit length** — it is always
/// constructed normalised by [`climb_aim_dir`]. Distinct from a position
/// ([`SimPos`]) and from a [`Direction::forward_step`] ground step: this is the full
/// 3D shot axis, including the vertical recoil tilt. Private inner + derived
/// `Deref` (house style); `z` is the `+z` ("up") component the climb tilts.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct AimDir(Vec3);

impl AimDir {
    /// The inner unit direction vector, by value.
    ///
    /// A convenience over the derived [`Deref`] for callers that want the [`Vec3`]
    /// itself (e.g. the cone sampler) without dereferencing — the vector is unit
    /// length by [`climb_aim_dir`]'s construction.
    #[must_use]
    pub const fn vec(self) -> Vec3 {
        self.0
    }
}

/// The per-stance muzzle height level-fraction for `stance`, read off `tuning`'s
/// [`crate::tuning::MuzzleHeights`] — no magnitude lives here.
const fn muzzle_height(stance: StanceKind, tuning: &CombatTuning) -> MuzzleHeight {
    let heights = &tuning.cone_stability.muzzle_heights;
    match stance {
        StanceKind::Prone => heights.prone,
        StanceKind::Crouching => heights.kneel,
        StanceKind::Standing => heights.stand,
    }
}

/// The per-stance **silhouette-top** level-fraction for `stance`, read off `tuning`'s
/// dedicated [`crate::tuning::SilhouetteTops`] (the silhouette-top home GTW-164 added
/// to [`CombatTuning`]) — no magnitude lives here. The stance→field mapping matches
/// [`muzzle_height`] (prone → `prone`, kneel/`Crouching` → `kneel`, stand → `stand`;
/// battle-space.md §"Stance / cover / muzzle / aim heights").
///
/// (Replaces the former `ProjectileBandEdges`-derived silhouette band-top, which
/// conflated silhouette/aim height with the round's clearance band edges.)
const fn silhouette_top(stance: StanceKind, tuning: &CombatTuning) -> SilhouetteTop {
    let tops = &tuning.cone_stability.silhouette_tops;
    match stance {
        StanceKind::Prone => tops.prone,
        StanceKind::Crouching => tops.kneel,
        StanceKind::Standing => tops.stand,
    }
}

/// The **band-top** level-fraction of a [`HeightBand`], derived from the tunable
/// [`ProjectileBandEdges`] (resolution.md §1 / AC #3): Low-top = `low_mid`,
/// Mid-top = `mid_high`, High-top = `1.0` (the storey ceiling). Never a hardcoded
/// magnitude beyond the storey-ceiling fraction `1.0`.
fn band_top_fraction(band: HeightBand, edges: ProjectileBandEdges) -> f32 {
    match band {
        HeightBand::Low => *edges.low_mid,
        HeightBand::Mid => *edges.mid_high,
        // The top of the HIGH band is the top of the storey — the 1.0 level-fraction
        // (a level-fraction, not a pixel): nothing within a storey sits above it.
        HeightBand::High => 1.0,
    }
}

/// The **band-bottom** level-fraction of a [`HeightBand`], derived from the tunable
/// [`ProjectileBandEdges`]: Low-bottom = `0.0` (the storey floor), Mid-bottom =
/// `low_mid`, High-bottom = `mid_high`. Paired with [`band_top_fraction`] this
/// gives each band its `[bottom, top)` level-fraction range.
fn band_bottom_fraction(band: HeightBand, edges: ProjectileBandEdges) -> f32 {
    match band {
        // The bottom of the LOW band is the storey floor — the 0.0 level-fraction.
        HeightBand::Low => 0.0,
        HeightBand::Mid => *edges.low_mid,
        HeightBand::High => *edges.mid_high,
    }
}

/// The **midpoint** level-fraction of a [`HeightBand`] — halfway between its
/// `[bottom, top)` edges, both derived from the tunable [`ProjectileBandEdges`]
/// (AC #4: "a cover-occupied cell aims at the object's own band midpoint, derived
/// from the band edges, never a literal"). Lies strictly inside the band's range.
fn band_midpoint_fraction(band: HeightBand, edges: ProjectileBandEdges) -> f32 {
    let bottom = band_bottom_fraction(band, edges);
    let top = band_top_fraction(band, edges);
    f32::midpoint(bottom, top)
}

/// Clamp a within-cell ground coordinate `coord` to stay **inside** the cell whose
/// integer corner is `corner` — the `[corner, corner + 1)` half-open cell, so the
/// clamped point can never floor into a neighbour (AC #2 "the clamp holds").
///
/// The lower bound is the corner itself; the upper bound is the largest f32 strictly
/// below `corner + 1.0` (so a point clamped to the top edge still buckets to this
/// cell under [`crate::metric::pos_to_cell`], which floors). No pixel — this is a
/// sim-unit (cell-unit) clamp.
fn clamp_within_cell(coord: f32, corner: f32) -> f32 {
    let upper = (corner + 1.0).next_down();
    coord.clamp(corner, upper)
}

/// The shooter's **3D muzzle point** as a [`SimPos`] (resolution.md §1
/// `muzzle_position`; battle-space.md §"Sub-cell precision on the ground plane").
///
/// The ground-plane x/y is the shooter cell's [`cell_center`] plus the per-facing
/// forward offset — the tunable [`crate::tuning::MuzzleForwardOffset`] cell-fraction
/// times the facing's [`Direction::forward_step`] unit step — **clamped** so the
/// muzzle's x/y can never leave the shooter's own cell (AC #2). The z is the storey
/// floor (the [`Level`] cast to `f32`) plus the per-stance muzzle **level-fraction**
/// ([`crate::tuning::MuzzleHeights`]). "Up" is `+z`.
///
/// `position` and `stance` decompose the spec's "shooter" into the per-field ECS
/// components this needs (cell + storey from [`Position`], posture from [`Stance`]) —
/// the crate's per-field house style — and `facing` is the [`Facing`] it looks along.
/// Every magnitude comes from `tuning`; zero pixels.
#[must_use]
pub fn muzzle_position(
    position: Position,
    facing: Facing,
    stance: Stance,
    tuning: &CombatTuning,
) -> SimPos {
    let key = *position;
    let cell = Cell::new(key.x, key.y);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a Position's z is a storey index in 0..MAX_LEVELS (8), so this u8 cast cannot truncate or wrap"
    )]
    let level = Level::new(key.z as u8);

    let center = cell_center(cell, level);
    let offset = *tuning.cone_stability.muzzle_forward_offset;
    let step = (*facing).forward_step();

    // cell_center + forward-offset along the facing, on the ground plane, then clamp
    // each axis so the muzzle stays strictly within the shooter's cell.
    let raw_x = offset.mul_add(step.x, center.x);
    let raw_y = offset.mul_add(step.y, center.y);
    #[expect(
        clippy::cast_precision_loss,
        reason = "grid coords are tiny (0..60); the f32 conversion of the integer corner is exact for this range"
    )]
    let muzzle_x = clamp_within_cell(raw_x, cell.x as f32);
    #[expect(
        clippy::cast_precision_loss,
        reason = "grid coords are tiny (0..60); the f32 conversion of the integer corner is exact for this range"
    )]
    let muzzle_y = clamp_within_cell(raw_y, cell.y as f32);

    // z = storey floor (the level cast to f32) + the per-stance muzzle level-fraction.
    let muzzle_z = f32::from(*level) + *muzzle_height(*stance, tuning);

    SimPos::new(muzzle_x, muzzle_y, muzzle_z)
}

/// The **aim point** a shooter centres the cone on, as a [`SimPos`] (resolution.md
/// §1 `target_aim_point`; battle-space.md §"Stance / cover / muzzle / aim heights").
///
/// The ground-plane x/y is the target cell's [`cell_center`]; the z is a
/// level-fraction derived per-case (AC #3/#4):
///
/// * **No cover faced** (`cover_band == None`) — a **ganger** target: the target's
///   `stance` selects its **per-stance silhouette-top** level-fraction from the
///   dedicated [`crate::tuning::SilhouetteTops`] tuning (prone → `prone`, kneel →
///   `kneel`, stand → `stand`), which is scaled by [`crate::tuning::AimHeightFrac`] —
///   `z = level + silhouette_top × aim_height_frac`. The silhouette top is its own
///   tuning home, NOT the round's [`ProjectileBandEdges`] clearance band edges.
/// * **Cover-occupied cell** (`cover_band == Some(band)`): aim at the cover's own
///   [`HeightBand`] **midpoint** level-fraction ([`band_midpoint_fraction`], derived
///   from the tunable [`ProjectileBandEdges`] band edges — cover height genuinely IS
///   band-based) — `z = level + band_midpoint`, so deliberately shooting a low crate
///   works at range.
///
/// `position` and `stance` are the target's per-field ECS components; `cover_band`
/// is the [`HeightBand`] of any cover occupying the target cell (e.g. read off a
/// [`crate::cover::CoverEntry`]), or `None` for a bare ganger target. Every magnitude
/// comes from `tuning`; zero pixels.
#[must_use]
pub fn target_aim_point(
    position: Position,
    stance: Stance,
    cover_band: Option<HeightBand>,
    tuning: &CombatTuning,
) -> SimPos {
    let key = *position;
    let cell = Cell::new(key.x, key.y);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a Position's z is a storey index in 0..MAX_LEVELS (8), so this u8 cast cannot truncate or wrap"
    )]
    let level = Level::new(key.z as u8);

    let center = cell_center(cell, level);
    let edges = tuning.projectile_band_edges;

    // A cover-occupied cell aims at the cover band's midpoint level-fraction (cover
    // height genuinely IS band-based); a bare ganger target aims at its per-stance
    // silhouette-top level-fraction (the dedicated SilhouetteTops tuning) × aim_height_frac.
    let aim_fraction = if let Some(band) = cover_band {
        band_midpoint_fraction(band, edges)
    } else {
        let top = *silhouette_top(*stance, tuning);
        top * *aim_height_frac(tuning)
    };

    SimPos::new(center.x, center.y, f32::from(*level) + aim_fraction)
}

/// The [`AimHeightFrac`] coefficient from `tuning` (the dimensionless fraction of a
/// ganger target's silhouette-top height the aim point pins). No magnitude here.
const fn aim_height_frac(tuning: &CombatTuning) -> AimHeightFrac {
    tuning.cone_stability.aim_height_frac
}

/// The **recoil-climb-tilted firing axis** — the unit muzzle→aim direction, rotated
/// **up** (`+z`) by `prior_shots × recoil_climb × recoil_growth` radians before the
/// cone is sampled (resolution.md §1a: round *i*'s "central axis tilts upward by
/// `prior_shots × recoil_climb × recoil_growth` radians", with `recoil_growth` the
/// E2.2 stability second-curve output, so a braced/prone shooter climbs strictly
/// less).
///
/// With **zero** prior shots the tilt angle is `0`, so the returned [`AimDir`] is the
/// **untilted** unit muzzle→aim axis exactly. With positive prior shots, growth, and
/// climb, the axis pitches up — its `+z` component rises monotonically — about the
/// horizontal axis perpendicular to the muzzle→aim direction (so the heading is
/// preserved and only the elevation changes). The result is **always unit length**.
///
/// `recoil_climb` is the tuning [`crate::tuning::RecoilClimb`]; `recoil_growth` is the
/// E2.2 [`RecoilGrowth`]; `prior_shots` is the burst's [`PriorShots`]. Angular tilt in
/// the cubic-voxel metric — zero pixels.
#[must_use]
pub fn climb_aim_dir(
    muzzle: SimPos,
    aim_point: SimPos,
    prior_shots: PriorShots,
    recoil_climb: RecoilClimb,
    recoil_growth: RecoilGrowth,
) -> AimDir {
    // The base muzzle→aim axis, normalised. A degenerate zero-length axis (muzzle ==
    // aim) falls back to the +X unit so the result stays unit length and panic-free.
    let raw = *aim_point - *muzzle;
    let base = raw.normalize_or_zero();
    let base = if base == Vec3::ZERO { Vec3::X } else { base };

    // The climb angle: prior_shots × recoil_climb × recoil_growth radians. Zero prior
    // shots → 0 radians → no tilt (the base axis returned exactly).
    let tilt = f32::from(*prior_shots) * *recoil_climb * *recoil_growth;
    if tilt == 0.0 {
        return AimDir(base);
    }

    // Tilt the axis UP by rotating `base` toward `+z` within the vertical plane the
    // two share, so only the elevation changes (heading preserved) and the tilt is
    // always upward regardless of heading. `up_perp` is the component of `+z`
    // orthogonal to `base` — the in-plane unit perpendicular pointing to the upward
    // side. For a base already pointing straight up/down `up_perp` is degenerate, so
    // the climb leaves it unchanged (a vertical shot has no heading to tilt).
    let up = Vec3::Z;
    let up_perp = up - base * up.dot(base);
    if up_perp.length_squared() <= 0.0 {
        return AimDir(base);
    }
    let up_perp = up_perp.normalize_or_zero();

    // Rotate `base` toward `up_perp` by `tilt` (an in-plane rotation toward +z), then
    // re-normalise to guarantee the unit-length invariant against f32 drift.
    let (sin, cos) = tilt.sin_cos();
    let rotated = base * cos + up_perp * sin;
    AimDir(rotated.normalize_or_zero())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ganger::{Direction, Facing, Position, Stance, StanceKind},
        metric::{Cell, CellLevel, Level, pos_to_cell},
    };

    /// A loose f32 tolerance for the geometry checks (`1/√2`, trig, normalisation
    /// are not exactly representable).
    const TOL: f32 = 1.0e-5;

    /// Build a `Position` at `(x, y)` on storey `level`.
    fn position(x: i32, y: i32, level: u8) -> Position {
        Position::new(CellLevel::new(Cell::new(x, y), Level::new(level)))
    }

    // --- AC #2: muzzle_position — forward-of-center in the facing dir, stays WITHIN
    // the cell (the clamp holds), z matches the per-stance level-fraction datum.

    #[test]
    fn muzzle_is_forward_of_center_in_the_facing_direction() {
        let tuning = CombatTuning::default();
        let pos = position(4, 7, 2);
        let center = cell_center(Cell::new(4, 7), Level::new(2));

        for dir in [
            Direction::North,
            Direction::NorthEast,
            Direction::East,
            Direction::SouthEast,
            Direction::South,
            Direction::SouthWest,
            Direction::West,
            Direction::NorthWest,
        ] {
            let muzzle = muzzle_position(
                pos,
                Facing::new(dir),
                Stance::new(StanceKind::Standing),
                &tuning,
            );
            let step = dir.forward_step();
            // The ground-plane displacement from center must point ALONG the facing's
            // forward step (positive dot product) — i.e. forward of center.
            let dx = muzzle.x - center.x;
            let dy = muzzle.y - center.y;
            let along = dx * step.x + dy * step.y;
            assert!(
                along > 0.0,
                "{dir:?}: muzzle must be forward of center along the facing (dot {along})",
            );
        }
    }

    #[test]
    fn muzzle_stays_within_the_shooter_cell_for_every_facing() {
        let tuning = CombatTuning::default();
        let cell = Cell::new(4, 7);
        let pos = position(4, 7, 2);

        for dir in [
            Direction::North,
            Direction::NorthEast,
            Direction::East,
            Direction::SouthEast,
            Direction::South,
            Direction::SouthWest,
            Direction::West,
            Direction::NorthWest,
        ] {
            let muzzle = muzzle_position(
                pos,
                Facing::new(dir),
                Stance::new(StanceKind::Standing),
                &tuning,
            );
            // The muzzle's (cell) must equal the shooter's cell — the clamp holds.
            let (muzzle_cell, _) = pos_to_cell(muzzle);
            assert_eq!(
                muzzle_cell, cell,
                "{dir:?}: the muzzle must stay within the shooter's cell",
            );
        }
    }

    #[test]
    fn muzzle_stays_within_cell_even_with_an_oversized_offset() {
        // A forward offset larger than half a cell would, unclamped, leave the cell
        // for some facings. The clamp must still keep it inside. Build a tuning with
        // a deliberately oversized cell-fraction offset (arbitrary, > 0.5).
        let mut tuning = CombatTuning::default();
        tuning.cone_stability.muzzle_forward_offset = crate::tuning::MuzzleForwardOffset::new(0.9);
        let cell = Cell::new(10, 10);
        let pos = position(10, 10, 0);

        for dir in [
            Direction::North,
            Direction::NorthEast,
            Direction::East,
            Direction::SouthEast,
            Direction::South,
            Direction::SouthWest,
            Direction::West,
            Direction::NorthWest,
        ] {
            let muzzle = muzzle_position(
                pos,
                Facing::new(dir),
                Stance::new(StanceKind::Standing),
                &tuning,
            );
            let (muzzle_cell, _) = pos_to_cell(muzzle);
            assert_eq!(
                muzzle_cell, cell,
                "{dir:?}: the clamp must hold even for an oversized offset",
            );
        }
    }

    #[test]
    fn muzzle_z_matches_the_per_stance_level_fraction_datum() {
        let tuning = CombatTuning::default();
        let level = 3u8;
        let pos = position(2, 2, level);
        let heights = &tuning.cone_stability.muzzle_heights;

        // For each stance, z must equal (level as f32) + the stance's muzzle
        // level-fraction (per-axis exact where the datum is exact).
        let cases = [
            (StanceKind::Prone, heights.prone),
            (StanceKind::Crouching, heights.kneel),
            (StanceKind::Standing, heights.stand),
        ];
        for (kind, height) in cases {
            let muzzle = muzzle_position(
                pos,
                Facing::new(Direction::North),
                Stance::new(kind),
                &tuning,
            );
            let expected = f32::from(level) + *height;
            assert_eq!(
                muzzle.z.to_bits(),
                expected.to_bits(),
                "{kind:?}: muzzle z must be level + per-stance muzzle level-fraction",
            );
        }
    }

    // --- AC #3: target_aim_point (ganger) — z above the floor is the per-stance
    // SilhouetteTops tuning × aim_height_frac (RELATION, not a literal magnitude).
    // These prove the dedicated SilhouetteTops field is what target_aim_point reads,
    // NOT the ProjectileBandEdges clearance band-tops.

    /// Build a `CombatTuning` whose per-stance silhouette tops are set to the given
    /// arbitrary (not shipped) level-fractions — so the assertions are by relation to
    /// the inputs, never pinning a magnitude.
    fn tuning_with_silhouette_tops(prone: f32, kneel: f32, stand: f32) -> CombatTuning {
        use crate::tuning::{SilhouetteTop, SilhouetteTops};
        let mut tuning = CombatTuning::default();
        tuning.cone_stability.silhouette_tops = SilhouetteTops {
            prone: SilhouetteTop::new(prone),
            kneel: SilhouetteTop::new(kneel),
            stand: SilhouetteTop::new(stand),
        };
        tuning
    }

    #[test]
    fn ganger_aim_z_is_per_stance_silhouette_top_times_aim_height_frac() {
        // Arbitrary distinct silhouette tops + an arbitrary aim_height_frac: each
        // stance's above-floor aim z must be ITS OWN SilhouetteTop × aim_height_frac.
        let mut tuning = tuning_with_silhouette_tops(0.2, 0.45, 0.85);
        tuning.cone_stability.aim_height_frac = AimHeightFrac::new(0.7);
        let frac = *tuning.cone_stability.aim_height_frac;
        let level = 2u8;
        let pos = position(5, 5, level);

        // prone → prone, kneel → Crouching, stand → Standing — each reads its own top.
        let cases = [
            (StanceKind::Prone, 0.2_f32),
            (StanceKind::Crouching, 0.45_f32),
            (StanceKind::Standing, 0.85_f32),
        ];
        for (kind, top) in cases {
            let aim = target_aim_point(pos, Stance::new(kind), None, &tuning);
            let expected = top.mul_add(frac, f32::from(level));
            assert!(
                (aim.z - expected).abs() < TOL,
                "{kind:?}: aim z {} should equal level + silhouette_top × aim_height_frac {expected}",
                aim.z,
            );
        }
    }

    #[test]
    fn ganger_aim_z_reads_each_stance_its_own_silhouette_top() {
        // RELATION: each stance maps to its DISTINCT SilhouetteTops field. With three
        // strictly-ordered tops, the above-floor aim z must be strictly ordered the
        // same way (prone < kneel < stand) — proving the per-stance mapping, not a
        // shared/band-derived value.
        let tuning = tuning_with_silhouette_tops(0.1, 0.5, 0.9);
        let level = 0u8; // on level 0, aim.z IS the above-floor fraction.
        let pos = position(3, 4, level);

        let z_prone = target_aim_point(pos, Stance::new(StanceKind::Prone), None, &tuning).z;
        let z_kneel = target_aim_point(pos, Stance::new(StanceKind::Crouching), None, &tuning).z;
        let z_stand = target_aim_point(pos, Stance::new(StanceKind::Standing), None, &tuning).z;
        assert!(
            z_prone < z_kneel && z_kneel < z_stand,
            "each stance must read its own SilhouetteTop (prone {z_prone} < kneel {z_kneel} < stand {z_stand})",
        );
    }

    #[test]
    fn ganger_aim_z_doubles_when_a_stances_silhouette_top_doubles() {
        // RELATION: doubling ONE stance's SilhouetteTop doubles that stance's
        // above-floor aim z (aim_height_frac fixed) — proving SilhouetteTop is the
        // multiplied source, with no hardcoded magnitude.
        let single = tuning_with_silhouette_tops(0.3, 0.6, 0.4);
        let doubled = tuning_with_silhouette_tops(0.6, 0.6, 0.4);
        let pos = position(0, 0, 0); // level 0 → aim.z IS the above-floor fraction.
        let stance = Stance::new(StanceKind::Prone);

        let z_single = target_aim_point(pos, stance, None, &single).z;
        let z_doubled = target_aim_point(pos, stance, None, &doubled).z;
        assert!(
            2.0f32.mul_add(-z_single, z_doubled).abs() < TOL,
            "doubling the prone SilhouetteTop must double the above-floor aim z: {z_doubled} vs 2×{z_single}",
        );
    }

    #[test]
    fn ganger_aim_z_scales_with_aim_height_frac() {
        // RELATION: doubling aim_height_frac doubles the ABOVE-floor aim fraction —
        // proving it multiplies the silhouette-top, no hardcoded magnitude.
        let mut low = tuning_with_silhouette_tops(0.3, 0.6, 0.95);
        low.cone_stability.aim_height_frac = AimHeightFrac::new(0.5);
        let mut high = tuning_with_silhouette_tops(0.3, 0.6, 0.95);
        high.cone_stability.aim_height_frac = AimHeightFrac::new(1.0);

        let pos = position(0, 0, 0);
        let stance = Stance::new(StanceKind::Standing);
        let z_low = target_aim_point(pos, stance, None, &low).z;
        let z_high = target_aim_point(pos, stance, None, &high).z;
        // On level 0 the z IS the above-floor fraction; high frac (×1.0) must be
        // exactly twice the low frac (×0.5) above the floor.
        assert!(
            2.0f32.mul_add(-z_low, z_high).abs() < TOL,
            "aim z above floor must scale linearly with aim_height_frac: {z_high} vs 2×{z_low}",
        );
    }

    #[test]
    fn ganger_aim_xy_is_cell_center() {
        let tuning = CombatTuning::default();
        let pos = position(9, 12, 1);
        let center = cell_center(Cell::new(9, 12), Level::new(1));
        let aim = target_aim_point(pos, Stance::new(StanceKind::Standing), None, &tuning);
        assert_eq!(aim.x.to_bits(), center.x.to_bits());
        assert_eq!(aim.y.to_bits(), center.y.to_bits());
    }

    // --- AC #4: target_aim_point (cover) — aim z lies within the band's
    // [bottom, top) level-fraction range, proving the midpoint stays inside its band.

    #[test]
    fn cover_aim_z_is_within_each_bands_fraction_range() {
        let tuning = CombatTuning::default();
        let level = 3u8;
        let pos = position(6, 6, level);
        let edges = tuning.projectile_band_edges;

        for band in [HeightBand::Low, HeightBand::Mid, HeightBand::High] {
            let aim = target_aim_point(pos, Stance::new(StanceKind::Standing), Some(band), &tuning);
            let above_floor = aim.z - f32::from(level);
            let bottom = band_bottom_fraction(band, edges);
            let top = band_top_fraction(band, edges);
            assert!(
                above_floor >= bottom && above_floor < top,
                "{band:?}: aim fraction {above_floor} must lie in [{bottom}, {top})",
            );
        }
    }

    #[test]
    fn cover_aim_z_is_the_band_midpoint() {
        // The midpoint is exactly halfway between the band's derived edges.
        let tuning = CombatTuning::default();
        let pos = position(1, 1, 0);
        let edges = tuning.projectile_band_edges;
        for band in [HeightBand::Low, HeightBand::Mid, HeightBand::High] {
            let aim = target_aim_point(pos, Stance::new(StanceKind::Standing), Some(band), &tuning);
            let expected = band_midpoint_fraction(band, edges);
            assert!(
                (aim.z - expected).abs() < TOL,
                "{band:?}: cover aim z {} should be the band midpoint {expected}",
                aim.z,
            );
        }
    }

    // --- AC #5: climb_aim_dir — zero prior shots points muzzle→aim exactly; climb
    // monotone +z with prior shots; unit length for all inputs.

    #[test]
    fn zero_prior_shots_points_muzzle_to_aim_exactly() {
        let muzzle = SimPos::new(2.0, 2.0, 1.0);
        let aim = SimPos::new(8.0, 5.0, 1.5);
        let dir = climb_aim_dir(
            muzzle,
            aim,
            PriorShots::first(),
            RecoilClimb::new(0.05),
            RecoilGrowth::new(0.5),
        );
        // The untilted unit muzzle→aim axis.
        let expected = (*aim - *muzzle).normalize_or_zero();
        let got = dir.vec();
        assert!(
            (got - expected).length() < TOL,
            "zero prior shots must yield the untilted muzzle→aim axis exactly: {got:?} vs {expected:?}",
        );
        // And it is unit length.
        assert!((got.length() - 1.0).abs() < TOL);
    }

    #[test]
    fn climb_increases_z_component_monotonically_with_prior_shots() {
        let muzzle = SimPos::new(0.0, 0.0, 1.0);
        // A target ahead and roughly level, so the base axis is near-horizontal and
        // a positive tilt clearly raises +z.
        let aim = SimPos::new(10.0, 0.0, 1.0);
        let climb = RecoilClimb::new(0.05);
        let growth = RecoilGrowth::new(0.5);

        let mut prev_z = f32::NEG_INFINITY;
        for shots in 0u16..6 {
            let dir = climb_aim_dir(muzzle, aim, PriorShots::new(shots), climb, growth);
            let z = dir.vec().z;
            assert!(
                z >= prev_z,
                "the +z component must be non-decreasing across prior shots: {z} after {prev_z} at {shots}",
            );
            if shots > 0 {
                assert!(
                    z > prev_z,
                    "a positive climb must strictly raise +z per prior shot at {shots}",
                );
            }
            // Always unit length.
            assert!(
                (dir.vec().length() - 1.0).abs() < TOL,
                "unit length at {shots} shots"
            );
            prev_z = z;
        }
    }

    #[test]
    fn climb_aim_dir_is_unit_length_for_arbitrary_inputs() {
        // Arbitrary muzzle/aim pairs and recoil inputs — the result is always unit.
        let cases = [
            (
                SimPos::new(1.0, 2.0, 0.5),
                SimPos::new(7.0, 9.0, 3.0),
                PriorShots::new(3),
            ),
            (
                SimPos::new(-4.0, 3.0, 2.0),
                SimPos::new(2.0, -1.0, 0.0),
                PriorShots::new(5),
            ),
            (
                SimPos::new(0.0, 0.0, 0.0),
                SimPos::new(0.0, 0.0, 6.0), // a near-vertical base axis
                PriorShots::new(2),
            ),
        ];
        for (muzzle, aim, shots) in cases {
            let dir = climb_aim_dir(
                muzzle,
                aim,
                shots,
                RecoilClimb::new(0.07),
                RecoilGrowth::new(0.6),
            );
            let len = dir.vec().length();
            assert!(
                (len - 1.0).abs() < TOL,
                "climb_aim_dir must be unit length for arbitrary inputs, got {len}",
            );
        }
    }

    #[test]
    fn degenerate_zero_axis_is_handled_without_panic() {
        // muzzle == aim: the base axis is zero-length; the fallback keeps the result
        // unit length rather than producing NaN or panicking.
        let p = SimPos::new(3.0, 3.0, 1.0);
        let dir = climb_aim_dir(
            p,
            p,
            PriorShots::first(),
            RecoilClimb::new(0.05),
            RecoilGrowth::new(0.5),
        );
        assert!((dir.vec().length() - 1.0).abs() < TOL);
    }

    #[test]
    fn aim_dir_newtype_derefs_to_inner() {
        let dir = climb_aim_dir(
            SimPos::new(0.0, 0.0, 0.0),
            SimPos::new(1.0, 0.0, 0.0),
            PriorShots::first(),
            RecoilClimb::new(0.0),
            RecoilGrowth::new(0.0),
        );
        // Deref and vec() reach the same inner Vec3.
        assert_eq!(*dir, dir.vec());
    }
}
