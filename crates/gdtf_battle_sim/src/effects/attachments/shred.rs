//! The **Shred** attachment effect (GTW-549 USER-REVIEW extra; GTW-558 one-file-per-effect)
//! — the isolated [`ApplyShred`] behaviour and the `impl` that raises the weapon's
//! armour-durability damage. No per-item magnitude newtype — its payload is the reused weapon
//! [`WeaponShred`](crate::weapon::WeaponShred).

use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::WeaponShred;

/// **Shred** — ADDS its [`WeaponShred`](crate::weapon::WeaponShred) to the weapon's
/// armour-durability damage (a serrated attachment).
///
/// Additive (saturating on the `i32` inner). A weapon with no shred stat is left unchanged.
/// USER-REVIEW extra (defensible default).
pub struct ApplyShred {
    /// The extra integrity damage this attachment adds.
    shred_bonus: WeaponShred,
}

impl ApplyShred {
    /// Build the shred effect from the [`WeaponShred`](crate::weapon::WeaponShred) it adds.
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

    /// `ApplyShred` RAISES the weapon's shred (additive).
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
