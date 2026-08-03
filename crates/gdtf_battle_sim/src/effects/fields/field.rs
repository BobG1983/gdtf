//! Catalog authoring for battlefield fields.
//!
//! [`FieldDef`] is the catalog side: damage, damage type, immune armor types, duration.
use bevy::reflect::TypePath;
use serde::Deserialize;

use crate::{
    effects::fields::{FieldDamage, FieldDuration, ImmuneArmorTypes},
    weapon::DamageType,
};

/// Authored definition of a placeable field type.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct FieldDef {
    pub damage:             FieldDamage,
    pub damage_type:        DamageType,
    pub immune_armor_types: ImmuneArmorTypes,
    pub duration:           FieldDuration,
}

impl FieldDef {
    #[must_use]
    pub const fn new(
        damage: FieldDamage,
        damage_type: DamageType,
        immune_armor_types: ImmuneArmorTypes,
        duration: FieldDuration,
    ) -> Self {
        Self {
            damage,
            damage_type,
            immune_armor_types,
            duration,
        }
    }
}
