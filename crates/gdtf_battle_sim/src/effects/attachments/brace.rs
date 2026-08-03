//! Brace attachment effect.

use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::Stable;

/// Marks the weapon as stable (braceable).
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
