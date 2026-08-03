use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    metric::CellLevel,
};

/// `#[serde(transparent)]` lets an authored cover-HP parse as a bare integer.
/// as a bare integer via `#[serde(transparent)]`.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct CoverHp(u32);

impl CoverHp {
        #[must_use]
    pub const fn new(hp: u32) -> Self {
        Self(hp)
    }

                #[must_use]
    pub fn saturating_sub(self, damage: CoverDamage) -> Self {
        Self(self.0.saturating_sub(*damage))
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum HeightBand {
        Low,
        Mid,
        High,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Destroyed(bool);

impl Destroyed {
        #[must_use]
    pub const fn new(destroyed: bool) -> Self {
        Self(destroyed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverEntry {
            pub current_hp:       CoverHp,
        pub max_hp:           CoverHp,
        pub height_band:      HeightBand,
        pub armor_protection: ArmorProtection,
        pub armor_hardness:   ArmorHardness,
        pub destroyed:        Destroyed,
}

impl CoverEntry {
                            #[must_use]
    pub const fn seeded(
        max_hp: CoverHp,
        height_band: HeightBand,
        armor_protection: ArmorProtection,
        armor_hardness: ArmorHardness,
    ) -> Self {
        Self {
            current_hp: max_hp,
            max_hp,
            height_band,
            armor_protection,
            armor_hardness,
            destroyed: Destroyed::new(false),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoverEvent {
        Damaged(CellLevel),
            Destroyed(CellLevel),
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverDamage(u32);

impl CoverDamage {
        #[must_use]
    pub const fn new(damage: u32) -> Self {
        Self(damage)
    }
}
