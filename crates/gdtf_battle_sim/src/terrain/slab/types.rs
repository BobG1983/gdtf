use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

use crate::armor::{ArmorHardness, ArmorProtection};

/// `#[serde(transparent)]` lets an authored slab-HP parse as a bare integer (the
/// integer via `#[serde(transparent)]`.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SlabHp(u32);

impl SlabHp {
        #[must_use]
    pub const fn new(hp: u32) -> Self {
        Self(hp)
    }

                #[must_use]
    pub fn saturating_sub(self, damage: SlabDamage) -> Self {
        Self(self.0.saturating_sub(*damage))
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabDestroyedFlag(bool);

impl SlabDestroyedFlag {
        #[must_use]
    pub const fn new(destroyed: bool) -> Self {
        Self(destroyed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabEntry {
            pub current_hp:       SlabHp,
        pub max_hp:           SlabHp,
        pub armor_protection: ArmorProtection,
        pub armor_hardness:   ArmorHardness,
        pub destroyed:        SlabDestroyedFlag,
}

impl SlabEntry {
                            #[must_use]
    pub const fn seeded(
        max_hp: SlabHp,
        armor_protection: ArmorProtection,
        armor_hardness: ArmorHardness,
    ) -> Self {
        Self {
            current_hp: max_hp,
            max_hp,
            armor_protection,
            armor_hardness,
            destroyed: SlabDestroyedFlag::new(false),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlabEvent {
        Damaged(crate::metric::CellLevel),
            Destroyed(crate::metric::CellLevel),
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabDamage(u32);

impl SlabDamage {
        #[must_use]
    pub const fn new(damage: u32) -> Self {
        Self(damage)
    }
}
