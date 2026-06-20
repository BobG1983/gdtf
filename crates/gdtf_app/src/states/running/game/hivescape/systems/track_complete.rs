use bevy::prelude::*;

use crate::states::running::game::hivescape::resources::HiveScapeComplete;

pub(in crate::states::running::game::hivescape) fn game_hivescape_complete(mut commands: Commands) {
    commands.insert_resource(HiveScapeComplete);
}
