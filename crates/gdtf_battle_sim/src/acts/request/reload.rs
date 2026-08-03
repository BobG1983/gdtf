//! Request to reload the actor's ranged weapon.

use bevy::prelude::{Entity, Message};

/// Reload request.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReloadRequested {
    /// Actor.
    pub actor: Entity,
}

impl ReloadRequested {
    /// Build a reload request.
    #[must_use]
    pub const fn new(actor: Entity) -> Self {
        Self { actor }
    }
}
