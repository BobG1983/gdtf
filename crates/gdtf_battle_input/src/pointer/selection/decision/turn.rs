use bevy::prelude::*;
use gdtf_battle_sim::{
    acts::SetFacingRequested,
    prelude::{Direction, Position},
};

use crate::{InspectTarget, selection::SelectedShooter};

#[must_use]
pub fn decide_turn(
    selected: &SelectedShooter,
    hovered: &InspectTarget,
    positions: &Query<&Position>,
) -> Option<SetFacingRequested> {
    let actor = (**selected)?;
    let target = hovered.hovered()?;
    let position = positions.get(actor).ok()?;
    let actor_cell = position.cell();
    let hovered_cell = target.cell();
    let facing = Direction::from_cells(actor_cell, hovered_cell)?;
    Some(SetFacingRequested::new(actor, facing))
}
