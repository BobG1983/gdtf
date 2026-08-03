//! Trait for applying an attachment effect to a weapon entity.

use bevy::prelude::EntityWorldMut;

/// Mutates a weapon entity when an attachment is fitted.
pub trait ApplyAttachmentEffect {
    /// Apply this effect onto `weapon`.
    fn apply_to_weapon(&self, weapon: &mut EntityWorldMut<'_>);
}
