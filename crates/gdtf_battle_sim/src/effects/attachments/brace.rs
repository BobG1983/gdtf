//! The **Brace** attachment effect (GTW-549 USER-REVIEW extra; GTW-558 one-file-per-effect)
use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::Stable;

/// A no-payload unit effect. USER-REVIEW extra (defensible default).
pub struct ApplyBrace;

impl ApplyAttachmentEffect for ApplyBrace {
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>) {
        weapon.insert(Stable::new(true));
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::{ApplyAttachmentEffect, ApplyBrace};
    use crate::weapon::Stable;

        #[test]
    fn brace_inserts_stable_tag() {
        let mut world = World::new();
        let weapon = world.spawn_empty().id();
        let mut entity = world.entity_mut(weapon);
        ApplyBrace.apply_to_weapon(&mut entity);
        let Some(stable) = entity.get::<Stable>() else {
            unreachable!("Brace must insert Stable");
        };
        assert!(**stable, "Brace fits Stable(true)");
    }
}
