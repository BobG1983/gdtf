use bevy::prelude::*;

use crate::scenes::running::game::battlescape::aftermath::display_after::resources::DisplayAfterComplete;

pub(in crate::scenes::running::game::battlescape::aftermath::display_after) fn cleanup(
    mut commands: Commands,
) {
    commands.remove_resource::<DisplayAfterComplete>();
}
