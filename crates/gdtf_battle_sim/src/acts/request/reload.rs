use bevy::prelude::{Entity, Message};

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReloadRequested {
        pub actor: Entity,
}

impl ReloadRequested {
        #[must_use]
    pub const fn new(actor: Entity) -> Self {
        Self { actor }
    }
}
