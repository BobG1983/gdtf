//! Mark newly downed gangers as bleeding out.

use bevy::prelude::{Changed, Commands, Entity, Query, Without};

use super::tick::BleedingOut;
use crate::ganger::LifeState;

type NewlyDowned<'world, 'state> =
    Query<'world, 'state, (Entity, &'static LifeState), (Changed<LifeState>, Without<BleedingOut>)>;

/// Insert [`BleedingOut`] when a ganger transitions to downed.
pub fn mark_downed_bleeding(newly_downed: NewlyDowned, mut commands: Commands) {
    for (entity, life) in &newly_downed {
        if *life == LifeState::Downed {
            commands.entity(entity).insert(BleedingOut);
        }
    }
}
