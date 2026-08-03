//! System: apply pending attachment effects, then clear the marker.

use bevy::prelude::{Commands, Entity, Query};

use super::AttachToWeaponExt;
use crate::weapon::PendingAttachments;

/// For each weapon with [`PendingAttachments`], apply effects and remove the component.
pub fn apply_pending_attachments(
    pending: Query<(Entity, &PendingAttachments)>,
    mut commands: Commands,
) {
    for (weapon, attachments) in &pending {
        for effect in attachments.effects() {
            commands.attach_to_weapon(weapon, effect.clone());
        }
        commands.entity(weapon).remove::<PendingAttachments>();
    }
}
