//! The **Penetration** attachment effect (GTW-549; GTW-558 one-file-per-effect) — the
//! isolated [`ApplyPenetration`] behaviour and the `impl` that raises the weapon's
//! penetration. No per-item magnitude newtype — its payload is the reused weapon
//! [`WeaponPunch`](crate::weapon::WeaponPunch).

use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::WeaponPunch;

/// **Penetration** — ADDS its [`WeaponPunch`](crate::weapon::WeaponPunch) to the weapon's
/// penetration (armour-piercing rounds, ignores more armour).
///
/// Additive (saturating on the `i32` inner via the public ctor). A weapon with no punch is
/// left unchanged.
pub struct ApplyPenetration {
    /// The extra penetration these rounds add.
    punch_bonus: WeaponPunch,
}

impl ApplyPenetration {
    /// Build the penetration effect from the [`WeaponPunch`](crate::weapon::WeaponPunch) it
    /// adds.
    #[must_use]
    pub const fn new(punch_bonus: WeaponPunch) -> Self {
        Self { punch_bonus }
    }
}

impl ApplyAttachmentEffect for ApplyPenetration {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        let Some(punch) = weapon.get::<WeaponPunch>() else {
            return;
        };
        let raised = WeaponPunch::new(punch.saturating_add(*self.punch_bonus));
        weapon.insert(raised);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyPenetration};
    use crate::weapon::WeaponPunch;

    /// `ApplyPenetration` RAISES the weapon's `WeaponPunch` (additive).
    #[test]
    fn penetration_raises_punch() {
        let mut world = World::new();
        let weapon = world.spawn(WeaponPunch::new(4)).id();
        let mut entity = world.entity_mut(weapon);
        ApplyPenetration::new(WeaponPunch::new(5)).apply_to_weapon(&mut entity);
        let Some(punch) = entity.get::<WeaponPunch>() else {
            unreachable!("the weapon must still carry WeaponPunch");
        };
        assert!(
            **punch > 4,
            "Penetration raises punch above the 4 baseline (got {})",
            **punch
        );
    }
}
