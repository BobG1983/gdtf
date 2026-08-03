use bevy::prelude::{Entity, Message};

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShoveRequested {
        pub shover: Entity,
        pub target: Entity,
            pub source: ShoveSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShoveSource {
                Deliberate,
            Weapon,
}

impl ShoveRequested {
            #[must_use]
    pub const fn new(shover: Entity, target: Entity) -> Self {
        Self {
            shover,
            target,
            source: ShoveSource::Deliberate,
        }
    }

                #[must_use]
    pub const fn new_weapon(shover: Entity, target: Entity) -> Self {
        Self {
            shover,
            target,
            source: ShoveSource::Weapon,
        }
    }
}
