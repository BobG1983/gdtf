//! Closed set of on-death effects for gangers and cover.

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::{ApplyExplode, ApplyLeaveField, ApplyOnDeathEffect, DeathFanOut, ExplodeDamage};
use crate::{
    effects::fields::FieldKey,
    metric::CellLevel,
    weapon::{DamageType, HitType},
};

/// Effect that fires when an entity or cover dies.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub enum OnDeathEffect {
    /// Explosion at the death cell.
    Explode {
        /// Hit geometry.
        hit_type: HitType,
        /// Damage amount.
        damage: ExplodeDamage,
        /// Damage channel.
        damage_type: DamageType,
    },
    /// Leave a field at the death cell.
    LeaveField {
        /// Field key to place.
        field: FieldKey,
    },
}

impl ApplyOnDeathEffect for OnDeathEffect {
    fn fan_at(&self, at: CellLevel, fan_out: &mut DeathFanOut<'_, '_, '_>) {
        match self {
            Self::Explode {
                hit_type, damage, ..
            } => ApplyExplode::new(*hit_type, *damage).fan_at(at, fan_out),
            Self::LeaveField { field } => ApplyLeaveField::new(field).fan_at(at, fan_out),
        }
    }
}
