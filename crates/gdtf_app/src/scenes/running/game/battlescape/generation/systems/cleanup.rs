use bevy::prelude::*;

use crate::scenes::running::game::battlescape::generation::resources::GenerationComplete;

pub(in crate::scenes::running::game::battlescape::generation) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<GenerationComplete>();
}
