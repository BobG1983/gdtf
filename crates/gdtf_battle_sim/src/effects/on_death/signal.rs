//! Death occurrence messages.

use bevy::prelude::{Entity, Message};

use crate::metric::CellLevel;

/// Something died at a cell (ganger or cover).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OnDeathOccurred {
    /// Entity that died (placeholder for cover).
    pub entity: Entity,
    /// Cell where death happened.
    pub at: CellLevel,
}

impl OnDeathOccurred {
    /// Death of a specific entity.
    #[must_use]
    pub const fn new(entity: Entity, at: CellLevel) -> Self {
        Self { entity, at }
    }

    /// Cover destruction (no real entity).
    #[must_use]
    pub const fn cover(at: CellLevel) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            at,
        }
    }
}
