use bevy::prelude::{Entity, Message};

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenDoorRequested {
            pub actor: Entity,
                pub door:  Entity,
}

impl OpenDoorRequested {
        #[must_use]
    pub const fn new(actor: Entity, door: Entity) -> Self {
        Self { actor, door }
    }
}
