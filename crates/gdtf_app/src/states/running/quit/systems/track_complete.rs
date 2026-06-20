use bevy::prelude::*;

use crate::states::running::quit::resources::QuitComplete;

pub(in crate::states::running::quit) fn quit_complete(mut commands: Commands) {
    commands.insert_resource(QuitComplete);
}
