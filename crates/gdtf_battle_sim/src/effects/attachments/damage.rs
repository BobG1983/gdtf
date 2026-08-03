//! Extra damage attachment effect.

use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::WeaponDamage;

/// Raises the weapon's base damage.
pub struct ApplyDamage {
    damage_bonus: WeaponDamage,
}

impl ApplyDamage {
    /// Build the applicator.
    #[must_use]
    pub const fn new(damage_bonus: WeaponDamage) -> Self {
        Self { damage_bonus }
    }
}

impl ApplyAttachmentEffect for ApplyDamage {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(damage) = weapon.get::<WeaponDamage>() else {
            return;
        };
        let raised = WeaponDamage::new(damage.saturating_add(*self.damage_bonus));
        weapon.insert(raised);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyDamage};
    use crate::weapon::WeaponDamage;

    #[test]
    fn damage_raises_base_damage() {
        let mut world = World::new();
        let weapon = world.spawn(WeaponDamage::new(10)).id();
        let mut entity = world.entity_mut(weapon);
        ApplyDamage::new(WeaponDamage::new(5)).apply_to_weapon(&mut entity);
        let Some(damage) = entity.get::<WeaponDamage>() else {
            unreachable!("WeaponDamage present");
        };
        assert!(**damage > 10, "Damage raises base damage above 10");
    }
}
