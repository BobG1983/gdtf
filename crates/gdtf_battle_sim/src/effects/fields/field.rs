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
    /// Damage per tick.
    pub damage:             FieldDamage,
    /// Damage channel.
    pub damage_type:        DamageType,
    /// Armor types immune to this field.
    pub immune_armor_types: ImmuneArmorTypes,
    /// How long the field lasts.
    pub duration:           FieldDuration,
}

impl FieldDef {
    /// Build a field definition.
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
