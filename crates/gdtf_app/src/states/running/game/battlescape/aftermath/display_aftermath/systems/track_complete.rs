use bevy::prelude::*;

use crate::states::running::game::battlescape::aftermath::display_aftermath::resources::DisplayAftermathComplete;

pub(in crate::states::running::game::battlescape::aftermath::display_aftermath) fn game_battlescape_aftermath_display_aftermath_complete(
    mut commands: Commands,
) {
    commands.insert_resource(DisplayAftermathComplete);
}
