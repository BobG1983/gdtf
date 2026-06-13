use bevy::prelude::*;

use crate::scenes::running::game::battlescape::generation::resources::GenerationComplete;

pub(in crate::scenes::running::game::battlescape::generation) fn game_battlescape_generation_complete(
    mut commands: Commands,
) {
    commands.insert_resource(GenerationComplete);
}
