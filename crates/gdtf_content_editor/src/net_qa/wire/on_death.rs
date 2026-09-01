//! One authored on-death effect on the wire: which variant, and its whole payload.

use gdtf_battle_sim::effects::on_death::OnDeathEffect;
use serde::{Deserialize, Serialize};

use super::{
    attachment::DamageTypeNet,
    fire_mode::HitTypeNet,
    weapon::{ExplodeDamageNet, FieldKeyNet},
};

/// Which on-death effect a form's own variant combo is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum OnDeathVariantNet {
    /// A blast at the death cell.
    Explode,
    /// A field left at the death cell.
    LeaveField,
}

impl OnDeathVariantNet {
    /// Mirror the variant an authored effect is on.
    pub(in crate::net_qa) const fn from_effect(effect: &OnDeathEffect) -> Self {
        match effect {
            OnDeathEffect::Explode { .. } => Self::Explode,
            OnDeathEffect::LeaveField { .. } => Self::LeaveField,
        }
    }
}

/// One authored on-death effect, the variant and every field its row draws.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum OnDeathEffectNet {
    /// A blast at the death cell.
    Explode {
        /// The blast's hit geometry.
        hit_type:    HitTypeNet,
        /// The flat damage each cell takes.
        damage:      ExplodeDamageNet,
        /// The channel the blast deals on.
        damage_type: DamageTypeNet,
    },
    /// A field left at the death cell.
    LeaveField {
        /// The catalog key of the field left behind.
        field: FieldKeyNet,
    },
}

impl OnDeathEffectNet {
    /// Mirror the sim's own effect.
    pub(in crate::net_qa) fn from_effect(effect: &OnDeathEffect) -> Self {
        match effect {
            OnDeathEffect::Explode {
                hit_type,
                damage,
                damage_type,
            } => Self::Explode {
                hit_type:    HitTypeNet::from_hit_type(*hit_type),
                damage:      ExplodeDamageNet::new(**damage),
                damage_type: DamageTypeNet::from_damage_type(*damage_type),
            },
            OnDeathEffect::LeaveField { field } => Self::LeaveField {
                field: FieldKeyNet::from_key(field),
            },
        }
    }

    /// Read a client's effect back as the sim's own.
    #[cfg(test)]
    pub(in crate::net_qa) fn to_effect(&self) -> OnDeathEffect {
        match self {
            Self::Explode {
                hit_type,
                damage,
                damage_type,
            } => OnDeathEffect::Explode {
                hit_type:    hit_type.to_hit_type(),
                damage:      damage.to_damage(),
                damage_type: damage_type.to_damage_type(),
            },
            Self::LeaveField { field } => OnDeathEffect::LeaveField {
                field: field.to_key(),
            },
        }
    }
}
