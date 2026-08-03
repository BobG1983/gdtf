use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, Message, MessageWriter},
};

use crate::acts::{FireRequested, movement::ReactionShotFired};

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InterruptDeclared {
        pub reactor:     Entity,
        pub interrupted: Entity,
}

impl InterruptDeclared {
        #[must_use]
    pub const fn new(reactor: Entity, interrupted: Entity) -> Self {
        Self {
            reactor,
            interrupted,
        }
    }
}

#[derive(SystemParam)]
pub struct InterruptSignals<'w> {
            pub(super) fire:     MessageWriter<'w, FireRequested>,
        pub(super) halt:     MessageWriter<'w, ReactionShotFired>,
            pub(super) declared: MessageWriter<'w, InterruptDeclared>,
}
