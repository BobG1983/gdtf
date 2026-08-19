//! Cover entry types: HP, height band, damage, and destroy events.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    metric::CellLevel,
    terrain::entity::TerrainPieceKind,
};

/// Cover hit points. Authored as a bare integer.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct CoverHp(u32);

impl CoverHp {
    /// Build from a raw value.
    #[must_use]
    pub const fn new(hp: u32) -> Self {
        Self(hp)
    }

    /// Subtract damage without going below zero.
    #[must_use]
    pub fn saturating_sub(self, damage: CoverDamage) -> Self {
        Self(self.0.saturating_sub(*damage))
    }
}

/// Height of cover relative to a standing combatant.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum HeightBand {
    /// Ankle to knee.
    Low,
    /// Knee to chest.
    Mid,
    /// Chest and above.
    High,
}

/// Whether this cover entry has been destroyed.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Destroyed(bool);

impl Destroyed {
    /// Wrap a destroyed flag.
    #[must_use]
    pub const fn new(destroyed: bool) -> Self {
        Self(destroyed)
    }
}

/// One cover placement at a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverEntry {
    /// Current HP.
    pub current_hp:       CoverHp,
    /// Starting HP.
    pub max_hp:           CoverHp,
    /// Height band of this cover.
    pub height_band:      HeightBand,
    /// Armor protection value.
    pub armor_protection: ArmorProtection,
    /// Armor hardness value.
    pub armor_hardness:   ArmorHardness,
    /// Destroyed flag.
    pub destroyed:        Destroyed,
    /// Which kind of terrain piece this entry holds.
    pub kind:             TerrainPieceKind,
}

impl CoverEntry {
    /// Fresh cover at full HP, standing in for a piece of the given kind.
    #[must_use]
    pub const fn seeded(
        max_hp: CoverHp,
        height_band: HeightBand,
        armor_protection: ArmorProtection,
        armor_hardness: ArmorHardness,
        kind: TerrainPieceKind,
    ) -> Self {
        Self {
            current_hp: max_hp,
            max_hp,
            height_band,
            armor_protection,
            armor_hardness,
            destroyed: Destroyed::new(false),
            kind,
        }
    }
}

/// Outcome of applying damage to cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoverEvent {
    /// Cover took damage but still stands.
    Damaged(CellLevel),
    /// Cover HP reached zero.
    Destroyed(CellLevel),
}

/// Damage applied to cover.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverDamage(u32);

impl CoverDamage {
    /// Wrap a damage amount.
    #[must_use]
    pub const fn new(damage: u32) -> Self {
        Self(damage)
    }
}
