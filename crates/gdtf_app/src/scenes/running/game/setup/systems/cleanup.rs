use bevy::prelude::*;

use crate::scenes::running::game::setup::resources::SetupComplete;

pub(in crate::scenes::running::game::setup) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<SetupComplete>();
}
