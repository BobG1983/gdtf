use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::DamageType;

pub struct ApplyDamageTypeOverride {
        damage_type: DamageType,
}

impl ApplyDamageTypeOverride {
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
