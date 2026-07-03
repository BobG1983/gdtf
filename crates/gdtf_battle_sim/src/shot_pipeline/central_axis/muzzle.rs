//! The §1 **muzzle point** — [`muzzle_position`] (cell-center + per-facing forward
//! offset, clamped within the cell, + per-stance muzzle level-fraction z).

use crate::{
    ganger::{Facing, Position, Stance, StanceKind},
    metric::{SimPos, cell_center},
    tuning::{CombatTuning, MuzzleHeight},
};

/// The per-stance muzzle height level-fraction for `stance`, read off `tuning`'s
/// [`crate::tuning::MuzzleHeights`] — no magnitude lives here.
///
/// `pub(crate)` so the level-aware LOS probe ([`crate::los`]) reuses the SAME
/// per-stance muzzle level-fraction [`muzzle_position`] reads for its facing-neutral
/// eye anchor (GTW-337 clause 3) — the eye is `cell_center + muzzle_height(stance)`
/// z, WITHOUT the per-facing forward XY offset [`muzzle_position`] adds, so the
/// observer's facing never moves its eye.
pub(crate) const fn muzzle_height(stance: StanceKind, tuning: &CombatTuning) -> MuzzleHeight {
    let heights = &tuning.cone_stability.muzzle_heights;
    match stance {
        StanceKind::Prone => heights.prone,
        StanceKind::Crouching => heights.kneel,
        StanceKind::Standing => heights.stand,
    }
}

/// Clamp a within-cell ground coordinate `coord` to stay **inside** the cell whose
/// integer corner is `corner` — the `[corner, corner + 1)` half-open cell, so the
/// clamped point can never floor into a neighbour (AC #2 "the clamp holds").
///
/// The lower bound is the corner itself; the upper bound is the largest f32 strictly
/// below `corner + 1.0` (so a point clamped to the top edge still buckets to this
/// cell under [`crate::metric::pos_to_cell`], which floors). No pixel — this is a
/// sim-unit (cell-unit) clamp.
pub(crate) fn clamp_within_cell(coord: f32, corner: f32) -> f32 {
    let upper = (corner + 1.0).next_down();
    coord.clamp(corner, upper)
}

/// The shooter's **3D muzzle point** as a [`SimPos`] (resolution.md §1
/// `muzzle_position`; battle-space.md §"Sub-cell precision on the ground plane").
///
/// The ground-plane x/y is the shooter cell's [`cell_center`] plus the per-facing
/// forward offset — the tunable [`crate::tuning::MuzzleForwardOffset`] cell-fraction
/// times the facing's [`crate::ganger::Direction::forward_step`] unit step —
/// **clamped** so the muzzle's x/y can never leave the shooter's own cell (AC #2). The
/// z is the storey floor (the [`Level`](crate::metric::Level) cast to `f32`) plus the
/// per-stance muzzle
/// **level-fraction** ([`crate::tuning::MuzzleHeights`]). "Up" is `+z`.
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
    // The canonical CellLevel::split decompose through Position's deref (GTW-565).
    let (cell, level) = position.split();

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
