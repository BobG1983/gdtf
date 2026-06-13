use bevy::prelude::*;

use crate::scenes::running::game::setup::resources::SetupComplete;

pub(in crate::scenes::running::game::setup) fn game_setup_complete(mut commands: Commands) {
    commands.insert_resource(SetupComplete);
}
