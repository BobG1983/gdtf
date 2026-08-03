use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::WeaponPunch;

pub struct ApplyPenetration {
        punch_bonus: WeaponPunch,
}

impl ApplyPenetration {
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
