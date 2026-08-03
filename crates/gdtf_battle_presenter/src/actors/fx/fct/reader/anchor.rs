use bevy::prelude::*;
use gdtf_battle_sim::{
    prelude::{Cell, Level, Position},
    resolve_coarse::ShotKind,
    shot_fired::ShotFired,
};

pub(in crate::actors::fx) fn anchor_cell(
    msg: &ShotFired,
    positions: &Query<&Position>,
) -> (Cell, Level) {
    if let ShotKind::Ganger(entity) = msg.kind
        && let Ok(pos) = positions.get(entity)
    {
        return pos.split();
    }
    (msg.impact_cell, msg.impact_level)
}
