use bevy::prelude::{Entity, Message};

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StabilizeDownedRequested {
        pub actor:  Entity,
        pub target: Entity,
}

impl StabilizeDownedRequested {
        #[must_use]
    pub const fn new(actor: Entity, target: Entity) -> Self {
        Self { actor, target }
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecuteDownedRequested {
        pub actor:  Entity,
        pub target: Entity,
}

impl ExecuteDownedRequested {
        #[must_use]
    pub const fn new(actor: Entity, target: Entity) -> Self {
        Self { actor, target }
    }
}
