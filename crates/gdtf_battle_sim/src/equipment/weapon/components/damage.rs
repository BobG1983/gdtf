//! Damage numbers and damage type.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// Bias toward fatal outcomes (authored; may be unused in current resolve).
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FatalBias(f32);

impl FatalBias {
    /// Wrap a bias value.
    #[must_use]
    pub const fn new(fatal_bias: f32) -> Self {
        Self(fatal_bias)
    }
}

/// Base damage points.
#[derive(
    Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct WeaponDamage(i32);

impl WeaponDamage {
    /// Wrap damage.
    #[must_use]
    pub const fn new(damage: i32) -> Self {
        Self(damage)
    }
}

/// Armor punch (penetration contribution).
#[derive(
    Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct WeaponPunch(i32);

impl WeaponPunch {
    /// Wrap punch.
    #[must_use]
    pub const fn new(punch: i32) -> Self {
        Self(punch)
    }
}

/// Armor shred (integrity wear contribution).
#[derive(
    Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct WeaponShred(i32);

impl WeaponShred {
    /// Wrap shred.
    #[must_use]
    pub const fn new(shred: i32) -> Self {
        Self(shred)
    }
}

/// Damage channel for armor matchups.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub enum DamageType {
    /// Shock.
    Shock,
    /// Blast / explosive.
    Blast,
    /// Chemical.
    Chem,
    /// Kinetic (default).
    #[default]
    Kinetic,
    /// Plasma.
    Plasma,
    /// Rend / tearing.
    Rend,
    /// Laser.
    Las,
}

impl DamageType {
    /// All variants.
    pub const ALL: [Self; 7] = [
        Self::Shock,
        Self::Blast,
        Self::Chem,
        Self::Kinetic,
        Self::Plasma,
        Self::Rend,
        Self::Las,
    ];
}
