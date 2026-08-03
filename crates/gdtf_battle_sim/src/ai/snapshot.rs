//! Snapshot of all gangers used for one enemy AI turn.

use bevy::{
    ecs::query::Has,
    prelude::{Deref, Entity, Query},
};

use crate::{
    acts::movement::WalkInProgress,
    ganger::{Aiming, Facing, Faction, LifeState, Position, Stance, Tu, TuMax},
    injuries::{HandsAvailable, InflictedInjuries, MovementCostFactor},
    metric::CellLevel,
};

/// Whether the ganger is mid-walk this frame.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MidWalk(bool);

impl MidWalk {
    pub(super) const fn new(walking: bool) -> Self {
        Self(walking)
    }
}

/// Query of all combatants for the enemy turn.
pub(super) type EnemyTurnGangers<'world, 'state> = Query<
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
        Has<WalkInProgress>,
        Option<&'static InflictedInjuries>,
    ),
>;

/// One ganger's fields needed by the AI brain.
#[derive(Clone, Copy)]
pub(super) struct GangerRow {
    pub(super) entity: Entity,
    pub(super) position: Position,
    pub(super) stance: Stance,
    pub(super) facing: Facing,
    pub(super) aiming: Aiming,
    pub(super) life: LifeState,
    pub(super) tu: Tu,
    pub(super) tu_max: TuMax,
    pub(super) faction: Faction,
    pub(super) walking: MidWalk,
    pub(super) hands: HandsAvailable,
    pub(super) factor: MovementCostFactor,
}

/// CellLevel from a position.
pub(super) fn row_cell_level(position: &Position) -> CellLevel {
    **position
}

/// Stable sort key for decision order.
pub(super) fn cell_order(position: &Position) -> (i32, i32, i32) {
    let key = ***position;
    (key.z, key.y, key.x)
}
