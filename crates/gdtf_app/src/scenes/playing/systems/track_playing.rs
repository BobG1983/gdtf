use bevy::prelude::*;

use crate::scenes::playing::resources::PlayingComplete;

pub(in crate::scenes::playing) fn playing_complete(mut commands: Commands) {
    commands.insert_resource(PlayingComplete);
}
