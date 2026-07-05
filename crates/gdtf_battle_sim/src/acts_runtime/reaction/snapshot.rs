//! The reaction-relevant ganger snapshot shape — the Copy [`ReactionRow`] the
//! trigger reads out of the live query, its deterministic ordering keys, and the
//! read-only snapshot query alias.

use bevy::prelude::{Entity, Query};

use crate::{
    ganger::{Aiming, Facing, Faction, LifeState, Position, Reactions, Stance, Tu, TuMax},
    metric::CellLevel,
};

/// One ganger's reaction-relevant snapshot — the Copy row [`reaction_trigger`](super::trigger::reaction_trigger) reads out of
/// the live query so the opposed check closes over plain data in a deterministic order,
/// never raw `Query` iteration order (`bevy-traps.md` #3, the `enemy_ai_turn` `GangerRow`
/// precedent).
///
/// Holds BOTH a reactor's gate inputs (its position / stance / facing / aim / life / TU /
/// `Reactions`) AND an actor's geometry (position / stance), since every ganger may be both
/// (faction-agnostic — the actor may be player- or enemy-controlled, C1).
#[derive(Clone, Copy)]
pub(super) struct ReactionRow {
    /// The ganger entity — the act emission's actor/reactor ref.
    pub(super) entity:    Entity,
    /// Its `(cell, level)` grid position — the eye / aim / arc datum.
    pub(super) position:  Position,
    /// Its stance — the eye / silhouette anchor for [`can_see`](crate::los::can_see).
    pub(super) stance:    Stance,
    /// Its facing — the arc datum for [`can_engage`](crate::acts::can_engage).
    pub(super) facing:    Facing,
    /// Its aim mode — the per-shot TU-premium selector for the interrupt shot's cost.
    pub(super) aiming:    Aiming,
    /// Its life state — only an Alive reactor watches; only an Alive ganger is a valid
    /// reaction target (a corpse/Downed body never reacts).
    pub(super) life:      LifeState,
    /// Its current TU pool — a reactor needs `Tu > 0` (unspent TU funds the interrupt).
    pub(super) tu:        Tu,
    /// Its round-start TU ceiling — the denominator of the §8 reaction score.
    pub(super) tu_max:    TuMax,
    /// Its gang — splits each `(reactor, actor)` pair into OPPOSING factions.
    pub(super) faction:   Faction,
    /// Its derived `Reactions` stat — the §8 score numerator + the cap input.
    pub(super) reactions: Reactions,
}

/// The `(cell, level)` key of a [`Position`].
pub(super) fn row_cell_level(position: &Position) -> CellLevel {
    **position
}

/// The `(level, y, x)` sort key of a [`Position`] — the deterministic total order the
/// trigger evaluates reactors in (`bevy-traps.md` #3 / the `brain.rs` `cell_order`
/// precedent), so the [`ReactionRng`](crate::rng::ReactionRng) draws consume in a reproducible order.
pub(super) fn cell_order(position: &Position) -> (i32, i32, i32) {
    let key = ***position;
    (key.z, key.y, key.x)
}

/// The read-only reaction snapshot query shape — every ganger's gate-relevant components,
/// factored into a `type` so the system signature stays under clippy's type-complexity gate
/// (the `EnemyTurnGangers` precedent).
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
