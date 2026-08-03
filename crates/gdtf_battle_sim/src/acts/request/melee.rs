use bevy::prelude::{Entity, Message};

use crate::{metric::CellLevel, resolve_hit::HpDamage, weapon::DamageType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeleeTarget {
                    Ganger(Entity),
                    Structure(CellLevel),
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeRequested {
            pub attacker: Entity,
        pub target:   MeleeTarget,
}

impl MeleeRequested {
            #[must_use]
    pub const fn new(attacker: Entity, target: Entity) -> Self {
        Self {
            attacker,
            target: MeleeTarget::Ganger(target),
        }
    }

            #[must_use]
    pub const fn new_structural(attacker: Entity, at: CellLevel) -> Self {
        Self {
            attacker,
            target: MeleeTarget::Structure(at),
        }
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeResolved {
            pub at:     CellLevel,
                pub damage: DamageType,
}

impl MeleeResolved {
            #[must_use]
    pub const fn new(at: CellLevel, damage: DamageType) -> Self {
        Self { at, damage }
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeStruck {
        pub attacker:  Entity,
        pub target:    Entity,
        pub hp_damage: HpDamage,
}

impl MeleeStruck {
        #[must_use]
    pub const fn new(attacker: Entity, target: Entity, hp_damage: HpDamage) -> Self {
        Self {
            attacker,
            target,
            hp_damage,
        }
    }
}
