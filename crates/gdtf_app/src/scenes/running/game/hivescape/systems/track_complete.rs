use bevy::prelude::*;

use crate::scenes::running::game::hivescape::resources::HiveScapeComplete;

pub(in crate::scenes::running::game::hivescape) fn game_hivescape_complete(mut commands: Commands) {
    commands.insert_resource(HiveScapeComplete);
}
