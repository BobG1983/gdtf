//! Result types produced by a hit resolution.

use bevy::prelude::Deref;

/// Damage that got past armor and can drive severity / wounds.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PenetratingDamage(i32);

impl PenetratingDamage {
    /// Build from a raw value.
    #[must_use]
    pub const fn new(pen: i32) -> Self {
        Self(pen)
    }
}

/// Hit points removed from the target.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HpDamage(i32);

impl HpDamage {
    /// Build from a raw value.
    #[must_use]
    pub const fn new(dmg: i32) -> Self {
        Self(dmg)
    }
}

/// Integrity removed from the armor piece that was hit.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntegrityWear(i32);

impl IntegrityWear {
    /// Build from a raw value.
    #[must_use]
    pub const fn new(wear: i32) -> Self {
        Self(wear)
    }
}

/// Integer damage magnitude used inside the formula.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DamageMagnitude(i32);

impl DamageMagnitude {
    /// Build from a raw value.
    #[must_use]
    pub const fn new(magnitude: i32) -> Self {
        Self(magnitude)
    }
}

/// Floating-point intermediate used for matchup scaling.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct DamageReal(f32);

impl DamageReal {
    /// Build from a raw value.
    #[must_use]
    pub const fn new(real: f32) -> Self {
        Self(real)
    }

    /// Read the inner float.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Full output of resolving a hit against one armor piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitResult {
    /// Damage that penetrated.
    pub penetrating: PenetratingDamage,
    /// HP that should be subtracted.
    pub hp_damage: HpDamage,
    /// Integrity that should be worn off the armor.
    pub wear: IntegrityWear,
}
