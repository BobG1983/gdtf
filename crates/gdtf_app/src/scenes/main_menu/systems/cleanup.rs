use bevy::prelude::*;

use crate::scenes::main_menu::resources::MainMenuComplete;

pub(in crate::scenes::main_menu) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<MainMenuComplete>();
}
