use bevy::prelude::*;

use crate::scenes::playing::resources::PlayingComplete;

pub(in crate::scenes::playing) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<PlayingComplete>();
}
