//! Request to open an adjacent door.

use bevy::prelude::{Entity, Message};

/// Open-door request.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenDoorRequested {
    /// Actor.
    pub actor: Entity,
    /// Door entity.
    pub door:  Entity,
}

impl OpenDoorRequested {
    /// Build an open-door request.
    #[must_use]
    pub const fn new(actor: Entity, door: Entity) -> Self {
        Self { actor, door }
    }
}
