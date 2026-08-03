use bevy::prelude::{Entity, Message};

use crate::{metric::CellLevel, weapon::DamageType};

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThrowGrenadeRequested {
            pub thrower: Entity,
            pub target:  CellLevel,
}

impl ThrowGrenadeRequested {
        #[must_use]
    pub const fn new(thrower: Entity, target: CellLevel) -> Self {
        Self { thrower, target }
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThrowResolved {
            pub at:     CellLevel,
            pub damage: DamageType,
}

impl ThrowResolved {
            #[must_use]
    pub const fn new(at: CellLevel, damage: DamageType) -> Self {
        Self { at, damage }
    }
}
