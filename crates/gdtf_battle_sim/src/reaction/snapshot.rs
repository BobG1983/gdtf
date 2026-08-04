//! Ganger snapshot used while evaluating reaction pairs.

use bevy::prelude::{Entity, Query};

use crate::{
    ganger::{Aiming, Facing, Faction, LifeState, Position, Reactions, Stance, Tu, TuMax},
    metric::CellLevel,
};

/// One ganger's reaction-relevant state for this pass.
#[derive(Clone, Copy)]
pub(super) struct ReactionRow {
    pub(super) entity:    Entity,
    pub(super) position:  Position,
    pub(super) stance:    Stance,
    pub(super) facing:    Facing,
    pub(super) aiming:    Aiming,
    pub(super) life:      LifeState,
    pub(super) tu:        Tu,
    pub(super) tu_max:    TuMax,
    pub(super) faction:   Faction,
    pub(super) reactions: Reactions,
}

pub(super) fn row_cell_level(position: &Position) -> CellLevel {
    **position
}

pub(super) fn cell_order(position: &Position) -> (i32, i32, i32) {
    let key = ***position;
    (key.z, key.y, key.x)
}

pub(super) type ReactionGangers<'world, 'state> = Query<
    'world,
    'state,
    (
        Entity,
        &'static Position,
        &'static Stance,
        &'static Facing,
        &'static Aiming,
        &'static LifeState,
        &'static Tu,
        &'static TuMax,
        &'static Faction,
        &'static Reactions,
    ),
>;
