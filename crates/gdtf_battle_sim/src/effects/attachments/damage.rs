//! The **Damage** attachment effect (GTW-549 USER-REVIEW extra; GTW-558 one-file-per-effect)
use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::WeaponDamage;

/// USER-REVIEW extra (defensible default).
pub struct ApplyDamage {
        damage_bonus: WeaponDamage,
}

impl ApplyDamage {
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
