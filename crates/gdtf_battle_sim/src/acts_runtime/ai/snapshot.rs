//! The brain's ganger snapshot shape — the read-only snapshot query alias, the Copy
//! [`GangerRow`] each frame reads out of it, and the deterministic ordering keys.

use bevy::{
    ecs::query::Has,
    prelude::{Entity, Query},
};

use crate::{
    ganger::{Aiming, Facing, Faction, LifeState, Position, Stance, Tu, TuMax},
    injuries::{HandsAvailable, InflictedInjuries, MovementCostFactor},
    metric::CellLevel,
    move_acts::WalkInProgress,
};

/// The brain's read-only ganger snapshot query shape — every ganger's brain-relevant
/// components plus its mid-walk flag, read out into a Copy [`GangerRow`] each frame
/// (factored into a `type` so the system signature stays under clippy's type-complexity
/// gate, the [`ShooterQuery`](crate::fire::ShooterQuery) precedent).
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
        // GTW-443: the enemy's injury ledger, read OPTIONALLY (an absent ledger = the
        // uninjured two-hands default), folded into the row's `hands` for the SHARED
        // can_fire hand-count gate — so the AI is refused a TwoHanded weapon below two
        // hands exactly as the player path is.
        Option<&'static InflictedInjuries>,
    ),
>;

/// A Copy snapshot of one ganger's brain-relevant state, read out of the live query so the
/// decision pass orders + closes over plain data — never raw `Query` iteration order
/// (`bevy-traps.md` #3 / GTW-70 §E).
#[derive(Clone, Copy)]
pub(super) struct GangerRow {
    /// The ganger entity — the act emissions' actor/target ref.
    pub(super) entity:   Entity,
    /// Its `(cell, level)` grid position.
    pub(super) position: Position,
    /// Its stance — the eye / silhouette anchor for `can_see`.
    pub(super) stance:   Stance,
    /// Its facing — the arc datum for `can_engage`.
    pub(super) facing:   Facing,
    /// Its aim mode — the per-shot TU premium selector.
    pub(super) aiming:   Aiming,
    /// Its life state — only an active (Alive) enemy acts; only an active observer sees.
    pub(super) life:     LifeState,
    /// Its current TU pool — the fire / move affordability budget.
    pub(super) tu:       Tu,
    /// Its round-start TU ceiling — the denominator of the per-shot TU charge.
    pub(super) tu_max:   TuMax,
    /// Its gang — splits acting enemies (`== active`) from targets (`!= active`).
    pub(super) faction:  Faction,
    /// Whether it is mid-walk (`Has<WalkInProgress>`) — skipped while busy, but it keeps
    /// the turn open (the §D.3 `busy` measure).
    pub(super) walking:  bool,
    /// Its available hand count (GTW-443) — folded from its injury ledger at snapshot
    /// time (an absent ledger = the uninjured two-hands default), fed to the SHARED
    /// `can_fire` hand-count gate.
    pub(super) hands:    HandsAvailable,
    /// Its movement-cost factor (GTW-444) — the "Hampered" slowdown folded from its injury
    /// ledger at snapshot time (an absent ledger = [`MovementCostFactor::IDENTITY`], `1.0`),
    /// fed to [`reachable_within`](crate::pathfinder::reachable_within) so a Hampered enemy's advance plan respects the SAME
    /// per-step slowdown the move dispatch will charge it.
    pub(super) factor:   MovementCostFactor,
}

/// The `(cell, level)` key of a [`Position`] — one [`Position`]→[`CellLevel`] deref.
pub(super) fn row_cell_level(position: &Position) -> CellLevel {
    **position
}

/// The `(level, y, x)` sort key of a [`Position`] — the deterministic total order the brain
/// visits actors in (`bevy-traps.md` #3 / GTW-70 §E).
pub(super) fn cell_order(position: &Position) -> (i32, i32, i32) {
    let key = ***position;
    (key.z, key.y, key.x)
}
