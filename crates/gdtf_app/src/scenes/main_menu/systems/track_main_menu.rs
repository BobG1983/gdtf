use bevy::prelude::*;

use crate::scenes::main_menu::resources::MainMenuComplete;

pub(in crate::scenes::main_menu) fn main_menu_complete(mut commands: Commands) {
    commands.insert_resource(MainMenuComplete);
}
