//! Stabilize or execute a downed target.

use bevy::prelude::{Entity, Message};

/// Stabilize a downed ally.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StabilizeDownedRequested {
    /// Actor performing the action.
    pub actor:  Entity,
    /// Downed target.
    pub target: Entity,
}

impl StabilizeDownedRequested {
    /// Build a stabilize request.
    #[must_use]
    pub const fn new(actor: Entity, target: Entity) -> Self {
        Self { actor, target }
    }
}

/// Execute a downed enemy.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecuteDownedRequested {
    /// Actor performing the action.
    pub actor:  Entity,
    /// Downed target.
    pub target: Entity,
}

impl ExecuteDownedRequested {
    /// Build an execute request.
    #[must_use]
    pub const fn new(actor: Entity, target: Entity) -> Self {
        Self { actor, target }
    }
}
