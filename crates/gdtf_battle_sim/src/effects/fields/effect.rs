//! Closed set of field consequences derived from a [`FieldDef`].

use bevy::prelude::Entity;
use serde::Deserialize;

use super::{
    ApplyDrain, ApplyDuration, ApplyFieldEffect, ApplyImmunity, DrainExempt, FieldDamage,
    FieldDuration, FieldExpired, FieldTurns, ImmuneArmorTypes, OccupantArmor, OccupantDrain,
};
use crate::{effects::fields::FieldDef, metric::CellLevel, weapon::DamageType};

/// One consequence a field applies each tick or on placement.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum FieldEffect {
    /// Drain HP from occupants.
    Drain {
        /// Damage per tick.
        damage:      FieldDamage,
        /// Damage channel.
        damage_type: DamageType,
    },
    /// Grant immunity for certain armor types.
    Immunity {
        /// Immune armor types.
        armor_types: ImmuneArmorTypes,
    },
    /// Lifetime of the field.
    Duration(FieldDuration),
}

impl FieldEffect {
    /// Expand a field def into its consequence list.
    #[must_use]
    pub fn consequences_of(def: &FieldDef) -> Vec<Self> {
        vec![
            Self::Drain {
                damage:      def.damage,
                damage_type: def.damage_type,
            },
            Self::Immunity {
                armor_types: def.immune_armor_types.clone(),
            },
            Self::Duration(def.duration),
        ]
    }

    fn with_behaviour<R>(&self, visit: impl FnOnce(&dyn ApplyFieldEffect) -> R) -> R {
        match self {
            Self::Drain { damage, .. } => visit(&ApplyDrain::new(*damage)),
            Self::Immunity { armor_types } => visit(&ApplyImmunity::new(armor_types)),
            Self::Duration(duration) => visit(&ApplyDuration::new(*duration)),
        }
    }
}

impl ApplyFieldEffect for FieldEffect {
    fn exempts_occupant(&self, armor: &OccupantArmor<'_, '_, '_>) -> DrainExempt {
        self.with_behaviour(|behaviour| behaviour.exempts_occupant(armor))
    }

    fn drain_occupant(
        &self,
        at: CellLevel,
        occupant: Entity,
        drain: &mut OccupantDrain<'_, '_, '_, '_, '_>,
    ) {
        self.with_behaviour(|behaviour| behaviour.drain_occupant(at, occupant, drain));
    }

    fn initial_countdown(&self) -> Option<FieldTurns> {
        self.with_behaviour(|behaviour| behaviour.initial_countdown())
    }

    fn count_down_one_turn(&self, remaining: &mut Option<FieldTurns>) -> FieldExpired {
        self.with_behaviour(|behaviour| behaviour.count_down_one_turn(remaining))
    }
}
