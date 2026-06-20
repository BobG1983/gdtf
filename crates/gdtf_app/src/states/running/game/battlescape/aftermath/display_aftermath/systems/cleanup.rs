use bevy::prelude::*;

use crate::states::running::game::battlescape::aftermath::display_aftermath::resources::DisplayAftermathComplete;

pub(in crate::states::running::game::battlescape::aftermath::display_aftermath) fn cleanup(
    mut commands: Commands,
) {
    commands.remove_resource::<DisplayAftermathComplete>();
}
