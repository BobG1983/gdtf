use bevy::prelude::*;

use crate::states::running::quit::resources::QuitComplete;

pub(in crate::states::running::quit) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<QuitComplete>();
}
