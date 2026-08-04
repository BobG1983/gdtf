//! Default HP and armor for uniform floor slabs.

use bevy::prelude::Deref;
use serde::Deserialize;

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    slab::SlabHp,
};

/// Default slab hit points.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct SlabDefaultHp(u16);

impl SlabDefaultHp {
    /// Wrap HP.
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

/// Full default set for slab structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct SlabDefaults {
    /// Default HP.
    pub default_hp:               SlabDefaultHp,
    /// Default protection.
    pub default_armor_protection: ArmorProtection,
    /// Default hardness.
    pub default_armor_hardness:   ArmorHardness,
}

impl SlabDefaults {
    /// HP as [`SlabHp`].
    #[must_use]
    pub fn hp(&self) -> SlabHp {
        SlabHp::new(u32::from(*self.default_hp))
    }

    /// Protection.
    #[must_use]
    pub const fn armor_protection(&self) -> ArmorProtection {
        self.default_armor_protection
    }

    /// Hardness.
    #[must_use]
    pub const fn armor_hardness(&self) -> ArmorHardness {
        self.default_armor_hardness
    }
}

impl SlabDefaults {
    /// Built-in defaults used when content omits slab stats.
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
