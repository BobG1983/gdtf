//! cover def authors (GTW-547, child GTW-41g; GTW-552 re-homes it into the
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::{ApplyExplode, ApplyLeaveField, ApplyOnDeathEffect, DeathFanOut, ExplodeDamage};
use crate::{
    effects::fields::FieldKey,
    metric::CellLevel,
    weapon::{DamageType, HitType},
};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub enum OnDeathEffect {
                                Explode {
                                        hit_type:    HitType,
                damage:      ExplodeDamage,
                                damage_type: DamageType,
    },
                    LeaveField {
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
