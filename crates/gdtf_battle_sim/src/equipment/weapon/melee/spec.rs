//! Authored melee weapon content.

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::{FightMode, MeleeDamageProfile, MeleeWeaponBundle, Reach};
use crate::{
    equipment::attachments::{FittedAttachments, WeaponSlots},
    weapon::{
        DamageType, FatalBias, Handedness, Shove, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred,
    },
};

/// Deserialized melee weapon definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TypePath)]
pub struct MeleeWeaponSpec {
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
    /// Reach (defaults to 1).
    #[serde(default)]
    pub reach:       Reach,
    /// Fight modes.
    pub fight_mode:  FightMode,
    /// Shove on hit.
    #[serde(default)]
    pub shove:       Shove,
    /// Attachment slots.
    #[serde(default)]
    pub slots:       WeaponSlots,
    /// Pre-fitted attachments.
    #[serde(default)]
    pub attachments: FittedAttachments,
}

impl MeleeWeaponSpec {
    /// Build a melee spawn bundle.
    #[must_use]
    pub fn into_bundle(self, name: WeaponName) -> MeleeWeaponBundle {
        MeleeWeaponBundle::new(
            name,
            MeleeDamageProfile::new(self.damage, self.punch, self.shred, self.damage_type),
            self.fatal_bias,
            self.handedness,
            self.reach,
            self.fight_mode,
            self.shove,
        )
    }
}
