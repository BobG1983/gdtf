//! The **Shove** attachment effect (GTW-549 USER-REVIEW extra; GTW-558 one-file-per-effect)
use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::Shove;

/// A no-payload unit effect. USER-REVIEW extra (defensible default).
pub struct ApplyShove;

impl ApplyAttachmentEffect for ApplyShove {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        weapon.insert(Shove::new(true));
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyShove};
    use crate::weapon::Shove;

        #[test]
    fn shove_inserts_shove_tag() {
        let mut world = World::new();
        let weapon = world.spawn_empty().id();
        let mut entity = world.entity_mut(weapon);
        ApplyShove.apply_to_weapon(&mut entity);
        let Some(shove) = entity.get::<Shove>() else {
            unreachable!("Shove must insert Shove");
        };
        assert!(**shove, "Shove fits Shove(true)");
    }
}
