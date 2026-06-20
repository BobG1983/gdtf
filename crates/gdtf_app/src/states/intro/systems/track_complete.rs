use bevy::prelude::*;

use crate::states::intro::resources::IntroComplete;

pub(in crate::states::intro) fn intro_complete(mut commands: Commands) {
    commands.insert_resource(IntroComplete);
}
