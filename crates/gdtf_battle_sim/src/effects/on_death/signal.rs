use bevy::prelude::{Entity, Message};

use crate::metric::CellLevel;

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OnDeathOccurred {
                pub entity: Entity,
            pub at:     CellLevel,
}

impl OnDeathOccurred {
        #[must_use]
    pub const fn new(entity: Entity, at: CellLevel) -> Self {
        Self { entity, at }
    }

                #[must_use]
    pub const fn cover(at: CellLevel) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            at,
        }
    }
}
