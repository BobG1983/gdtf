//! The public march entry point — [`march_vector`] flies one 3D shot ray through the
//! cubic-voxel grid (driving the [`dda`](super::dda) traversal) and reports the first
//! thing the round fails to clear (`docs/combat/resolution.md` §2 + §3).

use bevy::{math::Vec3, prelude::Entity};

use crate::{
    clearance::round_band_for_cell,
    cover::CoverLedger,
    march::{
        dda::{MAX_STEPS, MarchState, Step, impact_at},
        geom::{key_of, key_of_clamped, point_at, xy_in_grid, z_in_grid},
        result::{MarchKind, MarchResult},
    },
    metric::{CellLevel, SimPos, pos_to_cell},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
    tuning::CombatTuning,
};

/// March one 3D shot ray through the cubic-voxel grid and report the first thing the
/// round fails to clear (`docs/combat/resolution.md` §2 + §3) — the true 3-axis
/// voxel DDA.
///
/// The ray starts at the 3D `muzzle` [`SimPos`] and flies along `dir` (a unit-`Vec3`
/// shot direction; a non-unit `dir` is handled — only its direction matters). It
/// walks the 60×60×8 grid cell-by-cell / level-by-level in sim units (Amanatides–Woo).
///
/// At each voxel an **occupant** is banded: a ganger ([`OccupancyGrid::occupant`]
/// plus [`OccupancyGrid::occupant_band`]) or standing cover ([`CoverLedger`] entry,
/// excluded when destroyed via [`OccupancyGrid::is_blocked`] /
/// [`OccupancyGrid::is_cover_destroyed`]) is compared by
/// [`round_clears_occupant`](crate::clearance::round_clears_occupant) against the
/// round's [`round_band_for_cell`] band — an equal-or-lower round impacts
/// ([`MarchKind::Ganger`] / [`MarchKind::Cover`]), a strictly higher round sails
/// over. **Any** actor impacts — including the shooter's own gang (true friendly
/// fire) — by band alone, no exemption list. Each **z-boundary** step tests the
/// [`SurfaceGrid`] slab (one slab, both faces): [`SlabState::Present`](crate::surface::SlabState::Present)
/// stops the round ([`MarchKind::Slab`]); `Destroyed` / `Absent` passes. A **grid
/// exit** is top → clean sky [`MarchKind::Miss`]; bottom → [`MarchKind::Ground`];
/// lateral → [`MarchKind::Miss`].
///
/// **No target stop** — the round flies past the aim cell into whatever is behind it.
/// The **only** exception: the `shooter_cell` never blocks its own shot (its
/// occupant / cover is skipped). Degenerate marches (a `dir` that leaves the grid
/// immediately, a zero `dir`, an out-of-grid `muzzle`) return a [`MarchKind::Miss`]
/// gracefully — never a panic (AC #7). Every band read comes from `tuning` /
/// `cover` / `occupancy` / `surface` (no hardcoded grid or band constant beyond the
/// structural [`GRID_WIDTH`](crate::occupancy::GRID_WIDTH) /
/// [`GRID_HEIGHT`](crate::occupancy::GRID_HEIGHT) /
/// [`MAX_LEVELS`](crate::metric::MAX_LEVELS)).
///
/// `is_dead` is a read-only, RNG-free predicate the caller supplies to mark which
/// occupants are corpses: when the round would strike a ganger for which
/// `is_dead(entity)` is `true`, it passes **through** the corpse and continues to the
/// next blocker (next occupant / cover / wall / roof / floor / nothing) instead of
/// stopping. A live occupant — including a
/// [`LifeState::Downed`](crate::ganger::LifeState::Downed) one — still stops the
/// round; only [`LifeState::Dead`](crate::ganger::LifeState::Dead) is skipped
/// (GTW-317). The skip is deterministic and draws no RNG, so seeded replay is
/// unaffected.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "the march reads four independent, deliberately-separate grids (occupancy / \
              surface / cover / tuning) plus the muzzle, direction, shooter-cell exception, \
              and the GTW-317 dead-occupant predicate — each a distinct input the DDA must \
              see; bundling them into a struct would only obscure that they are read-only \
              and orthogonal"
)]
pub fn march_vector(
    muzzle: SimPos,
    dir: Vec3,
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
    shooter_cell: CellLevel,
    is_dead: impl Fn(Entity) -> bool,
) -> MarchResult {
    let (start_cell, start_level) = pos_to_cell(muzzle);
    let mut state = MarchState::new(
        start_cell.x,
        start_cell.y,
        i32::from(*start_level),
        muzzle,
        dir,
    );

    // Degenerate / out-of-grid start, or a zero direction: a graceful Miss at the
    // muzzle, no panic (AC #7).
    if dir == Vec3::ZERO || !xy_in_grid(state.vx, state.vy) || !z_in_grid(state.vz) {
        return MarchResult {
            kind:   MarchKind::Miss,
            at:     key_of_clamped(state.vx, state.vy, state.vz),
            band:   round_band_for_cell(muzzle, tuning),
            impact: muzzle,
        };
    }

    for _ in 0..MAX_STEPS {
        let here = key_of(state.vx, state.vy, state.vz);
        // The round's position as it sits in this voxel (its entry point) bands it at
        // this crossing (AC #1, #2). The shooter's own cell never blocks its own shot
        // (the one hard exception, AC #6) — skip the occupant / cover checks there.
        let here_point = point_at(muzzle, dir, state.entry_t);
        let round_band = round_band_for_cell(here_point, tuning);
        if here != shooter_cell
            && let Some(result) =
                impact_at(here, here_point, round_band, occupancy, cover, &is_dead)
        {
            return result;
        }

        // Step to the next voxel; an exit / intact slab ends the march here.
        match state.advance(muzzle, dir, surface, tuning, round_band) {
            Step::Continue => {}
            Step::Stopped(result) => return result,
        }
    }

    // The iteration cap was hit (only reachable for a pathological ray) — degrade to a
    // graceful Miss rather than spin (AC #7).
    let here_point = point_at(muzzle, dir, state.entry_t);
    MarchResult {
        kind:   MarchKind::Miss,
        at:     key_of_clamped(state.vx, state.vy, state.vz),
        band:   round_band_for_cell(here_point, tuning),
        impact: here_point,
    }
}
