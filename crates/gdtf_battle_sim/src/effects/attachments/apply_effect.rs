use bevy::prelude::EntityWorldMut;

pub trait ApplyAttachmentEffect {
                                    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>);
}
