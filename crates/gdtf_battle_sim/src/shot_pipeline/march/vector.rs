//! Straight-line ray march through the grid.

use bevy::{math::Vec3, prelude::Entity};

use crate::{
    clearance::{lower_band, round_band_for_cell},
    march::{
        dda::{MAX_STEPS, MarchState, Step, impact_at},
        geom::{MarchDir, VoxelIndex, key_of, key_of_clamped, point_at, xy_in_grid, z_in_grid},
        grids::MarchGrids,
        result::{MarchKind, MarchResult},
    },
    metric::{CellLevel, SimPos, pos_to_cell},
    tuning::CombatTuning,
};

/// March a direction from the muzzle until something is hit or the ray leaves the map.
#[must_use]
pub fn march_vector(
    muzzle: SimPos,
    dir: MarchDir,
    grids: MarchGrids<'_>,
    tuning: &CombatTuning,
    shooter_cell: CellLevel,
    is_dead: impl Fn(Entity) -> bool,
) -> MarchResult {
    let (start_cell, start_level) = pos_to_cell(muzzle);
    let mut state = MarchState::new(
        VoxelIndex::new(start_cell.x),
        VoxelIndex::new(start_cell.y),
        VoxelIndex::new(i32::from(*start_level)),
        muzzle,
        dir,
    );

    if *dir == Vec3::ZERO || !*xy_in_grid(state.vx, state.vy) || !*z_in_grid(state.vz) {
        return MarchResult {
            kind:   MarchKind::Miss,
            at:     key_of_clamped(state.vx, state.vy, state.vz),
            band:   round_band_for_cell(muzzle, tuning),
            impact: muzzle,
        };
    }

    for _ in 0..MAX_STEPS {
        let here = key_of(state.vx, state.vy, state.vz);
        let here_point = point_at(muzzle, dir, state.entry_t);
        let entry_band = round_band_for_cell(here_point, tuning);
        let exit_band = round_band_for_cell(state.exit_point(muzzle, dir), tuning);
        let test_band = lower_band(entry_band, exit_band);
        if here != shooter_cell
            && let Some(result) = impact_at(
                here,
                here_point,
                test_band,
                grids.occupancy,
                grids.surface,
                grids.cover,
                &is_dead,
            )
        {
            return result;
        }

        match state.advance(muzzle, dir, grids.surface, tuning, entry_band) {
            Step::Continue => {}
            Step::Stopped(result) => return result,
        }
    }

    let here_point = point_at(muzzle, dir, state.entry_t);
    MarchResult {
        kind:   MarchKind::Miss,
        at:     key_of_clamped(state.vx, state.vy, state.vz),
        band:   round_band_for_cell(here_point, tuning),
        impact: here_point,
    }
}
