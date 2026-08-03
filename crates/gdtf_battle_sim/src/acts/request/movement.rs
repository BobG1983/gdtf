use bevy::prelude::{Entity, Message};

use crate::metric::CellLevel;

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveRequested {
        pub actor: Entity,
        pub dest:  CellLevel,
}

impl MoveRequested {
        #[must_use]
    pub const fn new(actor: Entity, dest: CellLevel) -> Self {
        Self { actor, dest }
    }
}
