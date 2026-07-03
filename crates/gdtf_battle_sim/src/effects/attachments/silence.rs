//! The **Silence** attachment effect (GTW-549; GTW-558 one-file-per-effect) — the isolated
//! [`ApplySilence`] behaviour and the `impl` that fits the
//! [`Silenced`](crate::weapon::Silenced)`(true)` tag. A no-payload unit effect.

use bevy::prelude::EntityWorldMut;

use super::ApplyAttachmentEffect;
use crate::weapon::Silenced;

/// **Silence** — fits the [`Silenced`](crate::weapon::Silenced)`(true)` tag so the weapon's
/// shots propagate neither SUPPRESSION nor REACTION/REVEAL (a suppressor). PRESERVES the
/// GTW-542 [`Silenced`](crate::weapon::Silenced) component + BOTH its producer gates
/// unchanged — this effect only INSERTS the tag those gates already read.
///
/// A no-payload unit effect (the tag carries its own meaning).
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

    /// `ApplySilence` INSERTS the `Silenced(true)` tag both producer gates read.
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
