//! The level-aware coarse-geometry sight probe — [`has_los`], its borrow-view
//! inputs ([`Observer`] / [`Target`]), and the [`Sighted`] verdict.
//!
//! `has_los` WRAPS the one geometry truth ([`march_vector`](crate::march::march_vector))
//! and REUSES the shot pipeline's z-anchoring — it never re-implements LOS geometry
//! (GTW-337). See the module docs ([`super`]) for the two resolved fidelity decisions
//! (the facing-neutral eye + the exact compose-derived target aim band).

use bevy::{
    math::{Vec2, Vec3},
    prelude::{Component, Deref, Entity},
};

use crate::{
    central_axis::{clamp_within_cell, muzzle_height, target_aim_point},
    cover::CoverLedger,
    ganger::{Facing, Position, Stance, StanceKind},
    march::{MarchKind, march_vector},
    metric::{CellLevel, SimPos, cell_center},
    occupancy::{OccupancyGrid, StairEyeOffset},
    surface::SurfaceGrid,
    tuning::CombatTuning,
};

/// A sub-cell XY displacement of the observer's eye toward a cell edge — the wall-peek
/// offset (GTW-393).
///
/// Nudging the eye toward a cell edge lets an observer peer around a corner that
/// its cell-centre eye cannot clear. `Default` = `Vec2::ZERO` (no peek; the standard
/// centred eye). The XY analogue of [`StairEyeOffset`]'s z-lift. The inner `Vec2` is
/// **private** (no-bare-types rule): construct through [`PeekOffset::new`], read through
/// the derived [`Deref`].
///
/// **No automatic populator is built in GTW-393** — the component is non-zero only in
/// direct-set tests and under a future automatic positional populator (deferred, filed
/// as a follow-up ticket blocked-by GTW-393).
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Default)]
pub struct PeekOffset(Vec2);

impl PeekOffset {
    /// Build a wall-peek eye displacement — a sub-cell XY offset toward a cell edge.
    /// `Vec2::ZERO` is the identity (no peek); `Default` constructs it too.
    #[must_use]
    pub const fn new(displacement: Vec2) -> Self {
        Self(displacement)
    }
}

/// The **watcher** of a sight probe — a facing-aware borrow-view over the per-field
/// ganger components the eye anchor needs (`docs/combat/resolution.md` §1 muzzle;
/// the FOV epic GTW-13's observer eye).
///
/// Mirrors the shot pipeline's [`Shooter`](crate::aim::Shooter) borrow-view house
/// style: refs assembled from the live ganger components, not a stored struct. The
/// eye is anchored from [`position`](Observer::position) + [`stance`](Observer::stance);
/// [`facing`](Observer::facing) is carried for completeness (the omni-directional
/// eye is **facing-neutral**, so the eye anchor does NOT read it — see [`has_los`]),
/// reflecting that an observer is a directional entity even though its FOV is a disc.
///
/// [`stair_eye_offset`](Observer::stair_eye_offset) carries the GTW-390 authored
/// stair-tile lift for the observer's current cell — looked up from
/// [`OccupancyGrid::stair_eye_offset_at`] and threaded in by the caller so the
/// pure geometry probe stays resource-free.
#[derive(Debug, Clone, Copy)]
pub struct Observer<'a> {
    /// The watcher's `(cell, level)` grid position — the eye's x/y/level datum.
    pub position:         &'a Position,
    /// The watcher's stance — selects the per-stance muzzle level-fraction the eye
    /// sits at (a prone watcher's eye is lower than a standing one's).
    pub stance:           &'a Stance,
    /// The watcher's facing — the direction it looks along. Carried to mirror the
    /// directional shooter view; **not** read by the facing-neutral eye anchor.
    pub facing:           &'a Facing,
    /// The authored stair-tile eye-lift for the observer's cell (GTW-390) — looked
    /// up from [`OccupancyGrid::stair_eye_offset_at`] at the call site and threaded
    /// in here so the pure geometry probe never reads a resource. A non-stair cell
    /// carries `StairEyeOffset(0.0)` (no lift). The stance gate (Prone → 0.0) is
    /// applied inside `eye_anchor` so this field always carries the raw grid value.
    pub stair_eye_offset: StairEyeOffset,
    /// The wall-peek XY eye displacement (GTW-393) — a sub-cell nudge toward a cell
    /// edge, clamped inside `eye_anchor` so the eye stays within the observer's own
    /// cell. `PeekOffset::default()` (= `Vec2::ZERO`) is the centred, no-peek eye.
    pub peek_offset:      PeekOffset,
}

/// The **watched** entity of a sight probe — a borrow-view over the per-field ganger
/// components the target aim anchor needs.
///
/// Mirrors the shot pipeline's [`TargetGeometry`](mod@crate::fire) borrow-view house
/// style. The aim z is resolved the EXACT way the shot pipeline resolves it (the
/// `cover.peek().or_else(occupant_band)` band fed into
/// [`target_aim_point`](crate::central_axis::target_aim_point)), so eye-vs-aim
/// anchoring — and the asymmetry that falls out of it — matches a live shot exactly.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    /// The watched ganger's `(cell, level)` grid position — the aim's x/y/level datum.
    pub position: &'a Position,
    /// The watched ganger's stance — the fallback silhouette posture
    /// [`target_aim_point`](crate::central_axis::target_aim_point) reads when no band
    /// is published (the no-cover, no-occupant-band case).
    pub stance:   &'a Stance,
}

/// Whether one ganger can SEE another along the coarse voxel geometry — the verdict
/// [`has_los`] returns (`true` = clear line of sight, `false` = blocked).
///
/// A named newtype over `bool` (no-bare-types: a sight verdict is a domain value,
/// not a bare boolean). Private inner + derived [`Deref`] (house style): read the
/// verdict through the `*` deref, construct it only through [`Sighted::new`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sighted(bool);

impl Sighted {
    /// Build a sight verdict — `true` for a clear line of sight, `false` for blocked.
    #[must_use]
    pub const fn new(sighted: bool) -> Self {
        Self(sighted)
    }
}

/// The level-aware coarse-geometry sight probe: can the [`Observer`] SEE the
/// [`Target`] along the cubic-voxel grid? (GTW-337, leaf 1 of the GTW-13 FOV epic.)
///
/// This WRAPS the one geometry truth — [`march_vector`](crate::march::march_vector),
/// the 3-axis voxel DDA — and REUSES the shot pipeline's z-anchoring; it never runs
/// a second LOS implementation (`docs/combat/resolution.md` §2 clearance). The probe
/// is pure: render-free, RNG-free, deterministic, no `&mut World` / `Commands`.
///
/// The eye and aim anchors carry two resolved fidelity decisions:
///
/// * **The EYE is facing-neutral.** The eye sits at the observer cell's
///   [`cell_center`] for x/y plus a per-stance muzzle level-fraction z
///   (`f32::from(level) + muzzle_height(stance)`), REUSING the same
///   [`MuzzleHeights`](crate::tuning::MuzzleHeights) tuning
///   [`muzzle_position`](crate::central_axis::muzzle_position) reads, but WITHOUT its
///   per-facing forward XY offset
///   ([`MuzzleForwardOffset`](crate::tuning::MuzzleForwardOffset) ×
///   [`forward_step`](crate::ganger::Direction::forward_step)). FOV is
///   omni-directional — a watcher facing away must still SEE for the squad sight
///   union — so the eye must not move with facing; only the *ray* is directional, and
///   sight is bounded by the Chebyshev disc elsewhere in the epic. The spec's
///   `_los_start` is a HEIGHT, not the offset muzzle point.
/// * **The AIM mirrors the shot pipeline EXACTLY.** The target aim band is resolved
///   the way [`TargetGeometry::compose`](mod@crate::fire) does it —
///   `cover.peek(&target_cell_level).map(|e| e.height_band).or_else(|| occupancy.occupant_band(&target_cell_level))`
///   — then fed into [`target_aim_point`](crate::central_axis::target_aim_point) for
///   the aim z. Passing a bare `cover.peek` instead would drop a no-cover banded
///   ganger into `target_aim_point`'s silhouette-top × `aim_height_frac` branch — a
///   DIFFERENT z than the live shot pipeline aims at.
///
/// The unit direction eye→aim is built and [`march_vector`](crate::march::march_vector)
/// is called **exactly once** (passing `observer_cell` so the watcher's own cell never
/// blocks, and reusing the march's dead-occupant predicate so corpses do not block
/// sight). Sight is **BLOCKED** iff the march stops on a blocker (a slab, cover, or a
/// non-target ganger) at a `(cell, level)` strictly **before** the target's; a
/// `Miss` / `Ground` past the target, or a stop AT the target cell, is **CLEAR**. A
/// co-located degenerate `from == to` is sighted, and an off-grid input never panics
/// — it degrades to a defined verdict through the wrapped march's own graceful paths.
///
/// `is_dead` is the same read-only, RNG-free corpse predicate
/// [`march_vector`](crate::march::march_vector) takes: a ganger for which
/// `is_dead(entity)` is `true` does not block sight (it is a corpse), while a living
/// — including [`Downed`](crate::ganger::LifeState::Downed) — occupant does.
#[must_use]
pub fn has_los(
    from: &Observer,
    to: &Target,
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
    is_dead: impl Fn(Entity) -> bool,
) -> Sighted {
    let observer_cell = cell_level_of(from.position);
    let target_cell = cell_level_of(to.position);

    // A co-located observer/target is degenerate — the watcher trivially sees its
    // own cell (clause 6). Resolve it directly so the zero-length ray below never
    // matters.
    if observer_cell == target_cell {
        return Sighted::new(true);
    }

    let eye = eye_anchor(from, tuning);
    let aim = aim_anchor(to, occupancy, cover, tuning);

    // The unit eye→aim direction. `normalize_or_zero` yields the zero vector for a
    // coincident eye/aim (which `march_vector` then handles as a graceful Miss — no
    // panic, clause 6/7); the degenerate same-cell case is already returned above.
    let dir: Vec3 = (*aim - *eye).normalize_or_zero();

    // The ONE geometry call (clause 5): march the eye→aim ray through the voxel grid.
    // The observer's own cell is exempt (clause 6 — pass `observer_cell`), and the
    // corpse predicate is reused verbatim so a `Dead` occupant does not block sight.
    let result = march_vector(
        eye,
        dir,
        occupancy,
        surface,
        cover,
        tuning,
        observer_cell,
        is_dead,
    );

    is_clear(&result, target_cell, eye, aim)
}

/// A convenience wrapper over [`has_los`] that probes with a peeked eye — the
/// observer's eye is nudged by `peek` toward a cell edge before the LOS march
/// (GTW-393 wall-peek).
///
/// Constructs `Observer { peek_offset: peek, ..*from }` (valid because
/// `Observer: Copy`) and calls [`has_los`] on the single geometry path — no
/// second geometry implementation. The clamp in `eye_anchor` guarantees the
/// peeked eye stays within the observer's own cell regardless of the displacement
/// magnitude.
///
/// The squad fog union ([`union_fov`](crate::visibility::union_fov)) uses the
/// centred eye (`PeekOffset::default()`) — peek only enters a targeted,
/// per-shot query such as this one.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "has_los_peeking forwards every argument has_los needs plus the explicit \
              peek displacement — the same arity justification as has_los and can_see; \
              bundling into a struct would only hide the arity without reducing it"
)]
pub fn has_los_peeking(
    from: &Observer,
    to: &Target,
    peek: PeekOffset,
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
    is_dead: impl Fn(Entity) -> bool,
) -> Sighted {
    let peeking = Observer {
        peek_offset: peek,
        ..*from
    };
    has_los(&peeking, to, occupancy, surface, cover, tuning, is_dead)
}

/// The eye-anchor [`SimPos`] for `observer` — the facing-neutral watcher eye
/// (GTW-337 clause 3, GTW-390 stair eye-offset, GTW-393 wall-peek XY offset).
///
/// x/y is the observer cell's [`cell_center`] nudged by the wall-peek XY displacement
/// ([`PeekOffset`]) and clamped so the eye can never leave the observer's own cell
/// (AC #2 of `clamp_within_cell`); z is
/// `f32::from(level) + muzzle_height(stance) + stair_offset(stance, stair_eye_offset)`
/// — the SAME per-stance muzzle level-fraction
/// [`muzzle_position`](crate::central_axis::muzzle_position) reads, with NO per-facing
/// forward XY offset (so the observer's facing never moves its eye), PLUS a
/// stance-gated authored stair-tile lift (GTW-390):
///
/// * **Prone** → stair offset = 0.0 (a prone ganger's eye is already at the floor).
/// * **Standing or Crouching** → stair offset = `*observer.stair_eye_offset`
///   (i.e. +0.5 for a stair tile, 0.0 for a flat floor).
///
/// The wall-peek clamp invariant: `pos_to_cell(peeked_eye).cell == observer_cell`
/// for ANY peek displacement, including absurd values — the clamp proof holds because
/// the clamped coordinate floors to the same cell corner under `pos_to_cell`.
///
/// `pub(super)` so the in-crate tests can assert the facing-neutral invariant
/// directly (rotating `facing` leaves the eye unchanged), the stair-offset
/// invariants (GTW-390), and the peek clamp invariants (GTW-393).
pub(super) fn eye_anchor(observer: &Observer, tuning: &CombatTuning) -> SimPos {
    // Position derefs to CellLevel; split() is the canonical (Cell, Level)
    // decompose (GTW-565) for the cell_center eye datum.
    let (cell, level) = observer.position.split();
    let center = cell_center(cell, level);
    // Base eye height: per-stance muzzle level-fraction (shared with the shot
    // pipeline; facing-neutral — no forward XY offset applied here).
    let base_z = f32::from(*level) + *muzzle_height(**observer.stance, tuning);
    // Stair eye-lift (GTW-390): Prone observers stay on the floor; Standing and
    // Crouching observers on an authored stair tile add the half-level lift so their
    // eye clears mid-cover the way it would on a physical staircase.
    let stair_z = match **observer.stance {
        StanceKind::Prone => 0.0,
        StanceKind::Standing | StanceKind::Crouching => *observer.stair_eye_offset,
    };
    // Wall-peek XY offset (GTW-393): nudge the eye toward a cell edge by the
    // authored PeekOffset, then clamp each axis so the eye can never floor into a
    // neighbouring cell (AC #2: `pos_to_cell(peeked_eye).cell == observer_cell`).
    let peek = *observer.peek_offset;
    #[expect(
        clippy::cast_precision_loss,
        reason = "grid coords are tiny (0..60); the f32 conversion of the integer \
                  corner is exact for this range"
    )]
    let eye_x = clamp_within_cell(center.x + peek.x, cell.x as f32);
    #[expect(
        clippy::cast_precision_loss,
        reason = "grid coords are tiny (0..60); the f32 conversion of the integer \
                  corner is exact for this range"
    )]
    let eye_y = clamp_within_cell(center.y + peek.y, cell.y as f32);
    SimPos::new(eye_x, eye_y, base_z + stair_z)
}

/// The target aim-anchor [`SimPos`] for `target` — resolved the EXACT way the shot
/// pipeline resolves it (GTW-337 clause 4).
///
/// The aim band is `cover.peek(&target_cell_level).map(|e| e.height_band).or_else(||
/// occupancy.occupant_band(&target_cell_level))` — the same band derivation
/// [`TargetGeometry::compose`](crate::fire) uses — fed into
/// [`target_aim_point`](crate::central_axis::target_aim_point). A no-cover banded
/// ganger therefore aims at the band-midpoint branch (matching a live shot), NOT the
/// bare-stance silhouette-top branch.
///
/// `pub(super)` so the in-crate shot-pipeline-parity test can assert the aim z EQUALS
/// `target_aim_point(.., occupant_band, ..)` directly.
pub(super) fn aim_anchor(
    target: &Target,
    occupancy: &OccupancyGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
) -> SimPos {
    let at = cell_level_of(target.position);
    let aim_band = cover
        .peek(&at)
        .map(|entry| entry.height_band)
        .or_else(|| occupancy.occupant_band(&at));
    target_aim_point(*target.position, *target.stance, aim_band, tuning)
}

/// The `(cell, level)` grid key a [`Position`] occupies.
fn cell_level_of(position: &Position) -> CellLevel {
    **position
}

/// Decide CLEAR vs BLOCKED from the march result (GTW-337 clause 5).
///
/// Sight is **BLOCKED** iff the march stopped on a blocker — a [`MarchKind::Slab`],
/// [`MarchKind::Cover`], or a [`MarchKind::Ganger`] — at a `(cell, level)` strictly
/// **before** the target's. A [`MarchKind::Miss`] / [`MarchKind::Ground`] (nothing
/// blocked before the target) is CLEAR, and a blocker stop AT the target cell is
/// CLEAR (the target does not occlude itself).
///
/// "Strictly before" is decided two ways that must BOTH hold for a blocker stop: the
/// stop cell is not the target cell, AND the impact point is strictly nearer the eye
/// than the aim point (so a blocker the march reports *behind* the target — the march
/// flies past the aim cell — does not count). Comparing squared distances avoids a
/// `sqrt` and is exact enough for a strict-less comparison.
///
/// **GTW-391 forward-note (deferred to GTW-70 AI engagement / GTW-38 reaction fire).**
/// A future DIRECT `has_los(observer, target_at_lower_Position)` caller (not yet built)
/// will hit a mismatch when the target is a non-prone stair occupant: the ray strikes
/// the upper cell first (`result.at == upper ≠ lower target_cell`) and this function's
/// `result.at == target_cell` check returns `false` → classified BLOCKED. When GTW-70
/// or GTW-38 land, extend the check:
///
/// ```rust,ignore
/// // After fetching the target's PrevSlot from the ECS:
/// let blocked = (result.at != target_cell)
///     && result.kind.is_blocker()
///     && target.upper_cell.map_or(true, |u| result.at != u)
///     && impact_d2 < aim_d2;
/// ```
///
/// The shipped `union_fov` fog-reveal consumer is NOT affected (it probes the upper
/// cell as its own candidate, so `target_cell == upper` → `is_clear` → CLEAR → revealed).
fn is_clear(
    result: &crate::march::MarchResult,
    target_cell: CellLevel,
    eye: SimPos,
    aim: SimPos,
) -> Sighted {
    match result.kind {
        // Nothing was failed-to-clear before the target — a clean line of sight.
        MarchKind::Miss | MarchKind::Ground => Sighted::new(true),
        // A slab / cover / ganger stop: it blocks sight only if it is strictly before
        // the target (a different cell AND nearer the eye than the aim point).
        MarchKind::Slab | MarchKind::Cover(_) | MarchKind::Ganger(_) => {
            if result.at == target_cell {
                // A stop AT the target cell is the target itself (or its own cover) —
                // it does not occlude the target. CLEAR.
                return Sighted::new(true);
            }
            let impact_d2 = (*result.impact - *eye).length_squared();
            let aim_d2 = (*aim - *eye).length_squared();
            // BLOCKED only when the blocker is strictly nearer than the aim point;
            // otherwise the march reported something BEHIND the target → still CLEAR.
            Sighted::new(impact_d2 >= aim_d2)
        }
    }
}
