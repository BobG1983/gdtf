use bevy::prelude::{Commands, Entity, EntityWorldMut};

use crate::effects::attachments::ApplyAttachmentEffect;

pub trait AttachToWeaponExt {
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
