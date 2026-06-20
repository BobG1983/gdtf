use bevy::prelude::*;

use crate::states::intro::resources::IntroComplete;

pub(in crate::states::intro) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<IntroComplete>();
}
