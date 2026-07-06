//! The move act request — [`MoveRequested`].

use bevy::prelude::{Entity, Message};

use crate::metric::CellLevel;

/// A **move** act was requested — step `actor` one cell to `dest`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), carrying the
/// [`Entity`] actor ref plus the destination [`CellLevel`] (the `(cell, level)` to step
/// to). The payload is OWNED and `Copy` ([`CellLevel`] is `Copy`), so the type has **no
/// lifetime parameter** — mirroring [`SetFacingRequested`](super::posture::SetFacingRequested) / [`SetStanceRequested`](super::posture::SetStanceRequested). The
/// actor is a Bevy [`Entity`] handle — framework plumbing, the only bare type the
/// no-bare-types rule permits in a payload; `dest` is the landed [`CellLevel`] newtype.
/// [`dispatch_move`](crate::acts::movement::dispatch_move) drains this and, per message, plans a
/// reachable affordable route and attaches a
/// [`WalkInProgress`](crate::acts::movement::WalkInProgress) the landed
/// [`advance_walk`](crate::acts::movement::advance_walk) walk drives, each step's TU cost being
/// the entered cell's terrain movement cost.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveRequested {
    /// The acting ganger to step.
    pub actor: Entity,
    /// The destination `(cell, level)` to step the actor to (one cell, no pathfinding).
    pub dest:  CellLevel,
}

impl MoveRequested {
    /// Build a move request for `actor` to step to `dest`.
    #[must_use]
    pub const fn new(actor: Entity, dest: CellLevel) -> Self {
        Self { actor, dest }
    }
}
