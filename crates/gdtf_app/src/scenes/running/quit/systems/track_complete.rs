use bevy::prelude::*;

use crate::scenes::running::quit::resources::QuitComplete;

pub(in crate::scenes::running::quit) fn quit_complete(mut commands: Commands) {
    commands.insert_resource(QuitComplete);
}
