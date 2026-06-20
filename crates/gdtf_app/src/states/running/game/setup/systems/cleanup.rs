use bevy::prelude::*;

use crate::states::running::game::setup::resources::SetupComplete;

pub(in crate::states::running::game::setup) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<SetupComplete>();
}
