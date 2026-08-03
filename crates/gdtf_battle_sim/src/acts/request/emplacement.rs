use bevy::prelude::{Entity, Message};

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnterEmplacementRequested {
                pub actor:       Entity,
                pub emplacement: Entity,
}

impl EnterEmplacementRequested {
        #[must_use]
    pub const fn new(actor: Entity, emplacement: Entity) -> Self {
        Self { actor, emplacement }
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExitEmplacementRequested {
            pub actor:       Entity,
                pub emplacement: Entity,
}

impl ExitEmplacementRequested {
        #[must_use]
    pub const fn new(actor: Entity, emplacement: Entity) -> Self {
        Self { actor, emplacement }
    }
}
