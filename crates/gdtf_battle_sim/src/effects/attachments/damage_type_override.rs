//! The **`DamageTypeOverride`** attachment effect (GTW-549; GTW-558 one-file-per-effect) —
//! the isolated [`ApplyDamageTypeOverride`] behaviour and the `impl` that overrides the
//! weapon's emitted [`DamageType`](crate::weapon::DamageType). No per-item magnitude newtype —
//! its payload is the reused weapon [`DamageType`](crate::weapon::DamageType).

use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::DamageType;

/// **`DamageTypeOverride`** — OVERRIDES the weapon's emitted
/// [`DamageType`](crate::weapon::DamageType) (a toxic / elemental coating, a matchup-wheel
/// re-key).
///
/// A full override (inserts the new [`DamageType`](crate::weapon::DamageType) regardless of
/// the prior value — an insert replaces the component).
pub struct ApplyDamageTypeOverride {
    /// The damage type this coating forces the weapon to emit.
    damage_type: DamageType,
}

impl ApplyDamageTypeOverride {
    /// Build the damage-type-override effect from the [`DamageType`](crate::weapon::DamageType)
    /// it forces.
    #[must_use]
    pub const fn new(damage_type: DamageType) -> Self {
        Self { damage_type }
    }
}

impl ApplyAttachmentEffect for ApplyDamageTypeOverride {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        weapon.insert(self.damage_type);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyDamageTypeOverride};
    use crate::weapon::DamageType;

    /// `ApplyDamageTypeOverride` REPLACES the weapon's emitted `DamageType`.
    #[test]
    fn damage_type_override_replaces_the_type() {
        let mut world = World::new();
        let weapon = world.spawn(DamageType::Kinetic).id();
        let mut entity = world.entity_mut(weapon);
        ApplyDamageTypeOverride::new(DamageType::Chem).apply_to_weapon(&mut entity);
        let Some(damage_type) = entity.get::<DamageType>() else {
            unreachable!("the weapon must still carry a DamageType");
        };
        assert_eq!(
            *damage_type,
            DamageType::Chem,
            "DamageTypeOverride replaces Kinetic with the authored Chem"
        );
    }
}
