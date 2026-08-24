//! Snapshot of all gangers used for one enemy AI turn.

use bevy::{
    ecs::query::{Has, QueryData},
    prelude::{Deref, Entity, Query},
};

use crate::{
    acts::movement::WalkInProgress,
    ganger::{
        Aiming, Facing, Faction, LifeState, Position, Stance, Suppressed, SuppressorCell, Tu, TuMax,
    },
    injuries::{HandsAvailable, InflictedInjuries, MovementCostFactor},
    metric::CellLevel,
    terrain::emplacement::Mounted,
};

/// Whether the ganger is mid-walk this frame.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MidWalk(bool);

impl MidWalk {
    pub(super) const fn new(walking: bool) -> Self {
        Self(walking)
    }
}

/// One combatant's columns, as the enemy turn reads them.
#[derive(QueryData)]
pub struct CombatantColumns {
    entity:     Entity,
    position:   &'static Position,
    stance:     &'static Stance,
    facing:     &'static Facing,
    aiming:     &'static Aiming,
    life:       &'static LifeState,
    tu:         &'static Tu,
    tu_max:     &'static TuMax,
    faction:    &'static Faction,
    walking:    Has<WalkInProgress>,
    injuries:   Option<&'static InflictedInjuries>,
    suppressed: Option<&'static Suppressed>,
    mounted:    Option<&'static Mounted>,
}

/// Query of all combatants for the enemy turn.
pub(super) type EnemyTurnGangers<'world, 'state> = Query<'world, 'state, CombatantColumns>;

/// One ganger's fields needed by the AI brain.
#[derive(Clone, Copy)]
pub(super) struct GangerRow {
    pub(super) entity:    Entity,
    pub(super) position:  Position,
    pub(super) stance:    Stance,
    pub(super) facing:    Facing,
    pub(super) aiming:    Aiming,
    pub(super) life:      LifeState,
    pub(super) tu:        Tu,
    pub(super) tu_max:    TuMax,
    pub(super) faction:   Faction,
    pub(super) walking:   MidWalk,
    pub(super) hands:     HandsAvailable,
    pub(super) factor:    MovementCostFactor,
    /// Where the fire pinning this ganger came from, when it is pinned at all.
    pub(super) pinned_by: Option<SuppressorCell>,
    /// The seat this ganger rides, when it rides one. `Mounted` itself is not `Copy`.
    pub(super) seat:      Option<Entity>,
}

/// Snapshot every combatant into decision rows for this turn.
pub(super) fn ganger_rows(gangers: &EnemyTurnGangers) -> Vec<GangerRow> {
    gangers
        .iter()
        .map(|columns| GangerRow {
            entity:    columns.entity,
            position:  *columns.position,
            stance:    *columns.stance,
            facing:    *columns.facing,
            aiming:    *columns.aiming,
            life:      *columns.life,
            tu:        *columns.tu,
            tu_max:    *columns.tu_max,
            faction:   *columns.faction,
            walking:   MidWalk::new(columns.walking),
            hands:     columns
                .injuries
                .map_or_else(HandsAvailable::default, InflictedInjuries::hands_available),
            factor:    columns.injuries.map_or(
                MovementCostFactor::IDENTITY,
                InflictedInjuries::movement_cost_factor,
            ),
            pinned_by: columns.suppressed.map(|suppressed| suppressed.from),
            seat:      columns.mounted.and_then(Mounted::emplacement),
        })
        .collect()
}

/// `CellLevel` from a position.
pub(super) fn row_cell_level(position: &Position) -> CellLevel {
    **position
}

/// Stable sort key for decision order.
pub(super) fn cell_order(position: &Position) -> (i32, i32, i32) {
    let key = ***position;
    (key.z, key.y, key.x)
}
