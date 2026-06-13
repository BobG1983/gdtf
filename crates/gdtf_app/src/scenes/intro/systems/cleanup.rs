use bevy::prelude::*;

use crate::scenes::intro::resources::IntroComplete;

pub(in crate::scenes::intro) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<IntroComplete>();
}
