use bevy::prelude::*;

use crate::states::running::game::setup::resources::SetupComplete;

pub(in crate::states::running::game::setup) fn game_setup_complete(mut commands: Commands) {
    commands.insert_resource(SetupComplete);
}
