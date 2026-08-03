//! Slabs are **uniform level structure**: a situation authors them as a bare
//! `(cell, level)` list with NO per-slab HP (unlike cover, whose `CoverSpawn` authors
use bevy::prelude::Deref;
use serde::Deserialize;

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    slab::SlabHp,
};

/// the seed site. Private inner + derived [`Deref`]; `#[serde(transparent)]` lets it
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct SlabDefaultHp(u16);

impl SlabDefaultHp {
        #[must_use]
    pub const fn new(hp: u16) -> Self {
        Self(hp)
    }
}

impl Default for SlabDefaultHp {
    fn default() -> Self {
        Self(120)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct SlabDefaults {
        pub default_hp:               SlabDefaultHp,
        pub default_armor_protection: ArmorProtection,
        pub default_armor_hardness:   ArmorHardness,
}

impl SlabDefaults {
            #[must_use]
    pub fn hp(&self) -> SlabHp {
        SlabHp::new(u32::from(*self.default_hp))
    }

            #[must_use]
    pub const fn armor_protection(&self) -> ArmorProtection {
        self.default_armor_protection
    }

        #[must_use]
    pub const fn armor_hardness(&self) -> ArmorHardness {
        self.default_armor_hardness
    }
}

impl SlabDefaults {
                                                pub const FALLBACK: Self = Self {
        default_hp:               SlabDefaultHp::new(120),
        default_armor_protection: ArmorProtection::new(4),
        default_armor_hardness:   ArmorHardness::new(2),
    };
}

impl Default for SlabDefaults {
    fn default() -> Self {
        Self::FALLBACK
    }
}
