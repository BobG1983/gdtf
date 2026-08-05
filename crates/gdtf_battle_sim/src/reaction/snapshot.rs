//! Ganger snapshot used while evaluating reaction pairs.

use bevy::prelude::{Entity, Query};

use super::ledger::PendingSpendLedger;
use crate::{
    ganger::{Aiming, Facing, Faction, LifeState, Position, Reactions, Stance, Tu, TuMax},
    magazine::Magazine,
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

/// The two gangers one reaction is evaluated between.
pub(super) struct ReactionPair<'a> {
    pub(super) actor:   &'a ReactionRow,
    pub(super) reactor: &'a ReactionRow,
}

/// The state every pair in this pass is judged against: the ganger snapshot and pending spends.
pub(super) struct ReactionPass<'a> {
    rows:   &'a [ReactionRow],
    ledger: &'a PendingSpendLedger,
}

impl<'a> ReactionPass<'a> {
    /// Pair the snapshot with the spends committed so far.
    pub(super) const fn new(rows: &'a [ReactionRow], ledger: &'a PendingSpendLedger) -> Self {
        Self { rows, ledger }
    }

    /// Time units after any pending spend.
    pub(super) fn tu_of(&self, reactor: Entity, settled: Tu) -> Tu {
        self.ledger.tu_of(reactor, settled)
    }

    /// Facing after any pending turn.
    pub(super) fn facing_of(&self, reactor: Entity, settled: Facing) -> Facing {
        self.ledger.facing_of(reactor, settled)
    }

    /// Magazine after any pending rounds spent.
    pub(super) fn magazine_of(&self, weapon: Entity, live: Magazine) -> Magazine {
        self.ledger.magazine_of(weapon, live)
    }

    /// Life state snapshotted at the start of the pass; unknown entities count as alive.
    pub(super) fn life_of(&self, entity: Entity) -> LifeState {
        self.rows
            .iter()
            .find(|row| row.entity == entity)
            .map_or(LifeState::Alive, |row| row.life)
    }
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
