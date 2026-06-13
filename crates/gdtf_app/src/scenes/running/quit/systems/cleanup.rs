use bevy::prelude::*;

use crate::scenes::running::quit::resources::QuitComplete;

pub(in crate::scenes::running::quit) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<QuitComplete>();
}
