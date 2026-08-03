//! Commands helper to queue attachment effect application.

use bevy::prelude::{Commands, Entity, EntityWorldMut};

use crate::effects::attachments::ApplyAttachmentEffect;

/// Extend [`Commands`] with attach-to-weapon.
pub trait AttachToWeaponExt {
    /// Queue applying `effect` onto `weapon`.
    fn attach_to_weapon(
        &mut self,
        weapon: Entity,
        effect: impl ApplyAttachmentEffect + Send + 'static,
    ) -> &mut Self;
}

impl AttachToWeaponExt for Commands<'_, '_> {
    fn attach_to_weapon(
        &mut self,
        weapon: Entity,
        effect: impl ApplyAttachmentEffect + Send + 'static,
    ) -> &mut Self {
        self.entity(weapon)
            .queue(move |mut entity: EntityWorldMut| {
                effect.apply_to_weapon(&mut entity);
            });
        self
    }
}
