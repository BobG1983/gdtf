use bevy::prelude::*;

use crate::scenes::running::menu::resources::MenuComplete;

pub(in crate::scenes::running::menu) fn menu_complete(mut commands: Commands) {
    commands.insert_resource(MenuComplete);
}
