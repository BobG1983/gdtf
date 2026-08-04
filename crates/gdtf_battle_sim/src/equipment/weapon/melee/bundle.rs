//! Spawn bundle for a melee weapon.

use bevy::prelude::Bundle;

use super::{FightMode, MeleeWeapon, Reach};
use crate::weapon::{
    DamageType, FatalBias, Handedness, Shove, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
};

/// Full component set for a spawned melee weapon.
#[derive(Bundle, Debug, Clone, PartialEq)]
pub struct MeleeWeaponBundle {
    /// Marker.
    pub marker:      MeleeWeapon,
    /// Name.
    pub name:        WeaponName,
    /// Damage.
    pub damage:      WeaponDamage,
    /// Punch.
    pub punch:       WeaponPunch,
    /// Shred.
    pub shred:       WeaponShred,
    /// Damage type.
    pub damage_type: DamageType,
    /// Fatal bias.
    pub fatal_bias:  FatalBias,
    /// Handedness.
    pub handedness:  Handedness,
    /// Reach.
    pub reach:       Reach,
    /// Fight modes.
    pub fight_mode:  FightMode,
    /// Shove.
    pub shove:       Shove,
}

/// Damage numbers group for melee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeDamageProfile {
    /// Damage.
    pub damage:      WeaponDamage,
    /// Punch.
    pub punch:       WeaponPunch,
    /// Shred.
    pub shred:       WeaponShred,
    /// Damage type.
    pub damage_type: DamageType,
}

impl MeleeDamageProfile {
    /// Build a profile.
    #[must_use]
    pub const fn new(
        damage: WeaponDamage,
        punch: WeaponPunch,
        shred: WeaponShred,
        damage_type: DamageType,
    ) -> Self {
        Self {
            damage,
            punch,
            shred,
            damage_type,
        }
    }
}

impl MeleeWeaponBundle {
    /// Assemble from name, damage, and handling fields.
    #[must_use]
    pub const fn new(
        name: WeaponName,
        damage: MeleeDamageProfile,
        fatal_bias: FatalBias,
        handedness: Handedness,
        reach: Reach,
        fight_mode: FightMode,
        shove: Shove,
    ) -> Self {
        Self {
            marker: MeleeWeapon,
            name,
            damage: damage.damage,
            punch: damage.punch,
            shred: damage.shred,
            damage_type: damage.damage_type,
            fatal_bias,
            handedness,
            reach,
            fight_mode,
            shove,
        }
    }
}
