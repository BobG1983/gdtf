//! Stability / brace bonus attachment effect.

use bevy::prelude::{Component, Deref, EntityWorldMut};
use serde::{Deserialize, Serialize};

use super::ApplyAttachmentEffect;

/// Brace bonus points granted by an attachment.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct WeaponBraceBonus(f32);

impl WeaponBraceBonus {
    /// Wrap a bonus.
    #[must_use]
    pub const fn new(bonus: f32) -> Self {
        Self(bonus)
    }

    /// Zero bonus.
    #[must_use]
    pub const fn none() -> Self {
        Self(0.0)
    }
}

/// Inserts a brace bonus onto the weapon.
pub struct ApplyStability {
    bonus: WeaponBraceBonus,
}

impl ApplyStability {
    /// Build the applicator.
    #[must_use]
    pub const fn new(bonus: WeaponBraceBonus) -> Self {
        Self { bonus }
    }
}

impl ApplyAttachmentEffect for ApplyStability {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        weapon.insert(self.bonus);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyStability, WeaponBraceBonus};

    #[test]
    fn stability_inserts_weapon_brace_bonus() {
        let mut world = World::new();
        let weapon = world.spawn_empty().id();
        let mut entity = world.entity_mut(weapon);
        ApplyStability::new(WeaponBraceBonus::new(12.0)).apply_to_weapon(&mut entity);
        let Some(bonus) = entity.get::<WeaponBraceBonus>() else {
            unreachable!("Stability must insert a WeaponBraceBonus component");
        };
        assert!(
            **bonus > 0.0,
            "Stability inserts a positive brace bonus (got {})",
            **bonus
        );
    }
}
