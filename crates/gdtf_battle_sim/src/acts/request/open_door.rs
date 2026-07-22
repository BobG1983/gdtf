//! The open-door act request — [`OpenDoorRequested`].

use bevy::prelude::{Entity, Message};

/// An **open-door** act was requested — `actor` opens the adjacent closed `door` (GTW-315).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying the acting
/// ganger [`Entity`] + the openable-piece (door / hatch) [`Entity`]. The player-only
/// contextual Open-Door button writes this from the input queue when the selected ganger is
/// adjacent to a CLOSED door (F4 player-only; detect offers CLOSED doors only).
/// [`dispatch_open_door`](crate::acts::open_door::dispatch_open_door) drains it and RE-GATES in the
/// sim (the input layer's offer is advisory, never authoritative): the actor exists + can
/// afford the [`OpenDoorTu`](crate::tuning::OpenDoorTu) leaf, and the `door` entity carries an
/// [`OpenState`](crate::terrain::openable::OpenState) that is CLOSED and is 8-adjacent to the
/// actor. On pass it spends the [`OpenDoorTu`](crate::tuning::OpenDoorTu) leaf off the actor and
/// writes a [`SetOpenable::toggle`](crate::terrain::openable::SetOpenable::toggle) for the door
/// — REUSING the GTW-503 open mechanism verbatim (it never flips
/// [`OpenState`](crate::terrain::openable::OpenState) directly). The door TOGGLING open then
/// clears its path + vision block through the existing GTW-501 / GTW-502 change-detection (the
/// documented one-frame settle).
///
/// The `actor` / `door` are Bevy [`Entity`] handles — framework plumbing, the only bare type
/// the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenDoorRequested {
    /// The acting ganger opening the door — the [`OpenDoorTu`](crate::tuning::OpenDoorTu) leaf
    /// is spent off its TU pool, and it is the 8-adjacency reference for the gate.
    pub actor: Entity,
    /// The openable terrain piece (door / hatch) to open — gated CLOSED + 8-adjacent, then
    /// flipped open through the shared GTW-503 [`SetOpenable`](crate::terrain::openable::SetOpenable)
    /// mechanism.
    pub door:  Entity,
}

impl OpenDoorRequested {
    /// Build an open-door request for `actor` opening `door`.
    #[must_use]
    pub const fn new(actor: Entity, door: Entity) -> Self {
        Self { actor, door }
    }
}
