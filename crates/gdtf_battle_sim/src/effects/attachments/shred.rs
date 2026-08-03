//! Extra shred attachment effect.

use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::WeaponShred;

/// Raises the weapon's shred value.
pub struct ApplyShred {
    shred_bonus: WeaponShred,
}

impl ApplyShred {
    /// Build the applicator.
    #[must_use]
    pub const fn new(shred_bonus: WeaponShred) -> Self {
        Self { shred_bonus }
    }
}

impl ApplyAttachmentEffect for ApplyShred {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(shred) = weapon.get::<WeaponShred>() else {
            return;
        };
        let raised = WeaponShred::new(shred.saturating_add(*self.shred_bonus));
        weapon.insert(raised);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyShred};
    use crate::weapon::WeaponShred;

    #[test]
    fn shred_raises_shred() {
        let mut world = World::new();
        let weapon = world.spawn(WeaponShred::new(2)).id();
        let mut entity = world.entity_mut(weapon);
        ApplyShred::new(WeaponShred::new(3)).apply_to_weapon(&mut entity);
        let Some(shred) = entity.get::<WeaponShred>() else {
            unreachable!("WeaponShred present");
        };
        assert!(**shred > 2, "Shred raises shred above 2");
    }
}
