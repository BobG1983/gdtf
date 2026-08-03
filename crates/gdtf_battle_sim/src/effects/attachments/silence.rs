use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::Silenced;

pub struct ApplySilence;

impl ApplyAttachmentEffect for ApplySilence {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        weapon.insert(Silenced::new(true));
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplySilence};
    use crate::weapon::Silenced;

        #[test]
    fn silence_inserts_silenced_tag() {
        let mut world = World::new();
        let weapon = world.spawn_empty().id();
        let mut entity = world.entity_mut(weapon);
        ApplySilence.apply_to_weapon(&mut entity);
        let Some(silenced) = entity.get::<Silenced>() else {
            unreachable!("Silence must insert a Silenced component");
        };
        assert!(**silenced, "Silence fits Silenced(true)");
    }
}
