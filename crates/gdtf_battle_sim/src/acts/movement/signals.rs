use bevy::prelude::{Entity, Message};

use crate::metric::Cell;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveRejection {
                        Unreachable,
                        Unaffordable,
                                            Suppressed,
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveRejected {
        pub actor:  Entity,
        pub reason: MoveRejection,
}

impl MoveRejected {
        #[must_use]
    pub const fn new(actor: Entity, reason: MoveRejection) -> Self {
        Self { actor, reason }
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MovementOccurred {
            pub actor: Entity,
        pub from:  Cell,
        pub to:    Cell,
}

impl MovementOccurred {
        #[must_use]
    pub const fn new(actor: Entity, from: Cell, to: Cell) -> Self {
        Self { actor, from, to }
    }
}
