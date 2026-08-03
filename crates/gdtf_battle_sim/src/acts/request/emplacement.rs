//! Enter or exit a weapon emplacement.

use bevy::prelude::{Entity, Message};

/// Enter an emplacement.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnterEmplacementRequested {
    /// Actor.
    pub actor: Entity,
    /// Emplacement entity.
    pub emplacement: Entity,
}

impl EnterEmplacementRequested {
    /// Build an enter request.
    #[must_use]
    pub const fn new(actor: Entity, emplacement: Entity) -> Self {
        Self { actor, emplacement }
    }
}

/// Exit an emplacement.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExitEmplacementRequested {
    /// Actor.
    pub actor: Entity,
    /// Emplacement entity.
    pub emplacement: Entity,
}

impl ExitEmplacementRequested {
    /// Build an exit request.
    #[must_use]
    pub const fn new(actor: Entity, emplacement: Entity) -> Self {
        Self { actor, emplacement }
    }
}
