use bevy::prelude::*;

use crate::states::running::game::battlescape::generation::resources::GenerationComplete;

pub(in crate::states::running::game::battlescape::generation) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<GenerationComplete>();
}
