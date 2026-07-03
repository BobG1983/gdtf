//! The **Damage** attachment effect (GTW-549 USER-REVIEW extra; GTW-558 one-file-per-effect)
//! — the isolated [`ApplyDamage`] behaviour and the `impl` that raises the weapon's base
//! damage. No per-item magnitude newtype — its payload is the reused weapon
//! [`WeaponDamage`](crate::weapon::WeaponDamage).

use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::WeaponDamage;

/// **Damage** — ADDS its [`WeaponDamage`](crate::weapon::WeaponDamage) to the weapon's base
/// damage (a brutal counterweight / hotter load).
///
/// Additive (saturating on the `i32` inner). A weapon with no damage stat is left unchanged.
/// USER-REVIEW extra (defensible default).
pub struct ApplyDamage {
    /// The extra base damage this attachment adds.
    damage_bonus: WeaponDamage,
}

impl ApplyDamage {
    /// Build the damage effect from the [`WeaponDamage`](crate::weapon::WeaponDamage) it adds.
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

    /// `ApplyDamage` RAISES the weapon's base damage (additive).
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
