//! Despawn the pieces `replace_destroyed_piece` marked, once everything has read them.

use bevy::prelude::{Commands, Entity, Query, With};

use super::components::ReplacedPiece;

/// Despawn every piece that has already been replaced.
pub fn despawn_replaced_piece(
    mut commands: Commands,
    replaced: Query<Entity, With<ReplacedPiece>>,
) {
    for entity in &replaced {
        commands.entity(entity).despawn();
    }
}
