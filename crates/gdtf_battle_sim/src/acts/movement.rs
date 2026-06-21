//! The **move** dispatch — drain each buffered [`MoveRequested`] message and run the
//! landed [`move_ganger`] verb once per message (E4 / GTW-234).
//!
//! No act logic is reimplemented: the system REUSES the landed
//! [`move_ganger`](crate::move_acts::move_ganger) verb verbatim (whose gates + the
//! destination-terrain TU cost hold end-to-end) and fetches the actor's components via a
//! Bevy query (`bevy-traps.md` #7 — no `&mut World`); it only READS the grid.

use bevy::prelude::{Entity, Message, MessageReader, MessageWriter, Query, Res};

use crate::{
    acts::request::MoveRequested,
    ganger::{LifeState, Position, Tu},
    metric::Cell,
    move_acts::{MoveOutcome, move_ganger},
    occupancy::OccupancyGrid,
    tuning::CombatTuning,
};

/// A **move occurred** — the combat-log signal that `actor` stepped from `from` to `to`
/// (GTW-328), emitted ONCE per [`MoveRequested`] whose move actually SUCCEEDS.
///
/// The combat-text LOG event for a move ("<name> moved <from> -> <to>") — the user-facing
/// announcement that a ganger changed cell. It is emitted ONLY on a real step
/// ([`MoveOutcome::Moved`]); a blocked / unaffordable / no-op move logs nothing. The
/// [`from`](MovementOccurred::from) cell is captured BEFORE the [`Position`] write and
/// [`to`](MovementOccurred::to) AFTER, so they are the actual pre/post ground cells. It
/// adds **no** act logic and re-resolves nothing — pure exposure of the move the verb
/// already performed.
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`crate::acts::ReloadResult`]. The [`actor`](MovementOccurred::actor) is a Bevy
/// [`Entity`] handle — framework plumbing, the only bare type the no-bare-types rule
/// permits in a payload; [`from`](MovementOccurred::from) / [`to`](MovementOccurred::to)
/// are the domain [`Cell`] newtype.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MovementOccurred {
    /// The ganger that stepped — resolved to a name by the combat-log presenter via
    /// `Query<&GangerName>`.
    pub actor: Entity,
    /// The ground [`Cell`] the actor stepped FROM (captured before the [`Position`] write).
    pub from:  Cell,
    /// The ground [`Cell`] the actor stepped TO (captured after the [`Position`] write).
    pub to:    Cell,
}

impl MovementOccurred {
    /// Build a movement-occurred signal for `actor` stepping from `from` to `to`.
    #[must_use]
    pub const fn new(actor: Entity, from: Cell, to: Cell) -> Self {
        Self { actor, from, to }
    }
}

/// The ground-plane [`Cell`] of a [`Position`] — its `(x, y)` (the `z` storey is dropped).
/// [`Position`] derefs to [`CellLevel`](crate::metric::CellLevel) → the inner `IVec3`; the
/// cell is its `x`/`y` (the [`crate::acts::fire`] `actor_cell` precedent).
fn position_cell(position: &Position) -> Cell {
    let key = ***position;
    Cell::new(key.x, key.y)
}

/// **Dispatch** buffered [`MoveRequested`] messages — drain each and run the landed
/// [`move_ganger`] verb once per message (E4 / GTW-234).
///
/// Queries the actor's `(&mut `[`Position`]`, &mut `[`Tu`]`, &`[`LifeState`]`)`, reads
/// the [`OccupancyGrid`] (for the gates AND the destination-terrain cost lookup) and the
/// [`CombatTuning`] resource (for the [`MoveCosts`](crate::tuning::MoveCosts) table), and
/// calls [`move_ganger`] — whose gates (liveness / in-bounds / not-blocked / unoccupied /
/// affordable) hold end-to-end, charging the DESTINATION terrain's move cost. REUSES the
/// landed verb verbatim. A message for an actor missing any queried component is skipped
/// (fail-closed, no panic — the `dispatch_set_*` precedent).
///
/// This dispatch ONLY READS the grid (`Res<OccupancyGrid>`) for the gates + the terrain
/// cost; it never writes it. The grid's slot maintenance is the landed
/// [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) reacting to the
/// `Changed<`[`Position`]`>` this verb produces — both sit in the
/// [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) set and compose
/// with no ambiguity (`dispatch_move`'s `&mut Position` writes, `sync_moved_gangers`'s
/// `&Position` reads it next).
pub fn dispatch_move(
    mut requests: MessageReader<MoveRequested>,
    mut actors: Query<(&'static mut Position, &'static mut Tu, &'static LifeState)>,
    grid: Res<OccupancyGrid>,
    tuning: Res<CombatTuning>,
    mut moves: MessageWriter<MovementOccurred>,
) {
    for request in requests.read() {
        let Ok((mut position, mut tu, &life)) = actors.get_mut(request.actor) else {
            continue;
        };
        // GTW-328: capture the FROM ground cell BEFORE the Position write, so the
        // combat-log signal carries the actual pre-move cell.
        let from = position_cell(&position);
        let outcome = move_ganger(
            &mut position,
            &mut tu,
            life,
            request.dest,
            &grid,
            &tuning.move_costs,
        );
        // GTW-328: emit the combat-log move signal ONLY on a real step — the TO ground
        // cell is read AFTER the write. A blocked / unaffordable / no-op move logs
        // nothing. No re-resolve, no RNG — pure exposure of the verb's effect.
        if outcome == MoveOutcome::Moved {
            let to = position_cell(&position);
            moves.write(MovementOccurred::new(request.actor, from, to));
        }
    }
}
