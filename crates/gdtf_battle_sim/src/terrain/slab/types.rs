//! Slab entry types: HP, damage, and destroy events.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

use crate::armor::{ArmorHardness, ArmorProtection};

/// Slab hit points. Authored as a bare integer.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SlabHp(u32);

impl SlabHp {
    /// Wrap an HP value.
    #[must_use]
    pub const fn new(hp: u32) -> Self {
        Self(hp)
    }

    /// Subtract damage without going below zero.
    #[must_use]
    pub fn saturating_sub(self, damage: SlabDamage) -> Self {
        Self(self.0.saturating_sub(*damage))
    }
}

/// Whether this slab has been destroyed.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabDestroyedFlag(bool);

impl SlabDestroyedFlag {
    /// Wrap a destroyed flag.
    #[must_use]
    pub const fn new(destroyed: bool) -> Self {
        Self(destroyed)
    }
}

/// One slab placement at a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabEntry {
    /// Current HP.
    pub current_hp:       SlabHp,
    /// Starting HP.
    pub max_hp:           SlabHp,
    /// Armor protection value.
    pub armor_protection: ArmorProtection,
    /// Armor hardness value.
    pub armor_hardness:   ArmorHardness,
    /// Destroyed flag.
    pub destroyed:        SlabDestroyedFlag,
}

impl SlabEntry {
    /// Fresh slab at full HP.
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

/// Outcome of applying damage to a slab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlabEvent {
    /// Slab took damage but still stands.
    Damaged(crate::metric::CellLevel),
    /// Slab HP reached zero.
    Destroyed(crate::metric::CellLevel),
}

/// Damage applied to a slab.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlabDamage(u32);

impl SlabDamage {
    /// Wrap a damage amount.
    #[must_use]
    pub const fn new(damage: u32) -> Self {
        Self(damage)
    }
}
