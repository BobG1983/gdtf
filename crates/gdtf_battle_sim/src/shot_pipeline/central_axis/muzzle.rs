//! Where the shot leaves the weapon.

use crate::{
    ganger::{Facing, Position, Stance, StanceKind},
    metric::{SimPos, SimUnit, cell_center},
    tuning::{CombatTuning, MuzzleHeight},
};

/// Muzzle height above the floor for the current stance.
pub(crate) const fn muzzle_height(stance: StanceKind, tuning: &CombatTuning) -> MuzzleHeight {
    let heights = &tuning.cone_stability.muzzle_heights;
    match stance {
        StanceKind::Prone => heights.prone,
        StanceKind::Crouching => heights.kneel,
        StanceKind::Standing => heights.stand,
    }
}

/// Keep a coordinate inside the cell it belongs to.
pub(crate) fn clamp_within_cell(coord: SimUnit, corner: SimUnit) -> SimUnit {
    let upper = (*corner + 1.0).next_down();
    SimUnit::new((*coord).clamp(*corner, upper))
}

/// World position of the muzzle for a shooter.
#[must_use]
pub fn muzzle_position(
    position: Position,
    facing: Facing,
    stance: Stance,
    tuning: &CombatTuning,
) -> SimPos {
    let (cell, level) = position.split();

    let center = cell_center(cell, level);
    let offset = *tuning.cone_stability.muzzle_forward_offset;
    let step = *(*facing).forward_step();

    let raw_x = offset.mul_add(step.x, center.x);
    let raw_y = offset.mul_add(step.y, center.y);
    let muzzle_x = *clamp_within_cell(SimUnit::new(raw_x), SimUnit::new(cell.x as f32));
    let muzzle_y = *clamp_within_cell(SimUnit::new(raw_y), SimUnit::new(cell.y as f32));

    let muzzle_z = f32::from(*level) + *muzzle_height(*stance, tuning);

    SimPos::new(muzzle_x, muzzle_y, muzzle_z)
}
