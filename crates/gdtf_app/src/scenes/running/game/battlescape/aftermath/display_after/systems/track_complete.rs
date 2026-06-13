use bevy::prelude::*;

use crate::scenes::running::game::battlescape::aftermath::display_after::resources::DisplayAfterComplete;

pub(in crate::scenes::running::game::battlescape::aftermath::display_after) fn game_battlescape_aftermath_display_after_complete(
    mut commands: Commands,
) {
    commands.insert_resource(DisplayAfterComplete);
}
