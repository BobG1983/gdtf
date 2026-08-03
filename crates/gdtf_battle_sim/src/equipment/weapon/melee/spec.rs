//! The melee **authoring spec** — the [`MeleeWeaponSpec`] an
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::{FightMode, MeleeDamageProfile, MeleeWeaponBundle, Reach};
use crate::{
    equipment::attachments::{AttachmentName, WeaponSlots},
    weapon::{
        DamageType, FatalBias, Handedness, Shove, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred,
    },
};

/// `#[serde(transparent)]` bare RON scalar (the [`crate::tuning`] / GTW-200 house
/// `#[serde(default)]` — an omitted `reach:` falls back to [`Reach::DEFAULT`] (`1`), so
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TypePath)]
pub struct MeleeWeaponSpec {
        pub damage:      WeaponDamage,
        pub punch:       WeaponPunch,
            pub shred:       WeaponShred,
        pub damage_type: DamageType,
        pub fatal_bias:  FatalBias,
        pub handedness:  Handedness,
        /// `#[serde(default)]` so an omitted field falls back to [`Reach::DEFAULT`] (`1`).
    #[serde(default)]
    pub reach:       Reach,
                pub fight_mode:  FightMode,
        /// connecting melee strike. `#[serde(default)]` so an omitted `shove:` field falls
        /// `#[serde(default)]` precedent), so a melee weapon that never authors it keeps
        #[serde(default)]
    pub shove:       Shove,
                /// `#[serde(default)]` — an omitted field is the EMPTY declaration (bare fists take no
            #[serde(default)]
    pub slots:       WeaponSlots,
            /// [`attachments`](crate::weapon::WeaponSpec::attachments) mirror. `#[serde(default)]`
                        #[serde(default)]
    pub attachments: Vec<AttachmentName>,
}

impl MeleeWeaponSpec {
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
