use bevy::prelude::*;

use crate::scenes::intro::resources::IntroComplete;

pub(in crate::scenes::intro) fn intro_complete(mut commands: Commands) {
    commands.insert_resource(IntroComplete);
}
