use bevy::prelude::*;

use crate::scenes::running::game::battlescape::aftermath::display_aftermath::resources::DisplayAftermathComplete;

pub(in crate::scenes::running::game::battlescape::aftermath::display_aftermath) fn cleanup(
    mut commands: Commands,
) {
    commands.remove_resource::<DisplayAftermathComplete>();
}
