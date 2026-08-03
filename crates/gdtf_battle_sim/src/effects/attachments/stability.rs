use bevy::prelude::{Component, Deref, EntityWorldMut};
use serde::{Deserialize, Serialize};

use super::ApplyAttachmentEffect;

/// A `#[derive(Component)]` (no-bare-types: private inner + derived [`Deref`];
/// `#[serde(transparent)]` so it authors as a bare RON scalar — `Stability(12.0)`). The
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct WeaponBraceBonus(f32);

impl WeaponBraceBonus {
            #[must_use]
    pub const fn new(bonus: f32) -> Self {
        Self(bonus)
    }

                #[must_use]
    pub const fn none() -> Self {
        Self(0.0)
    }
}

pub struct ApplyStability {
        bonus: WeaponBraceBonus,
}

impl ApplyStability {
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
