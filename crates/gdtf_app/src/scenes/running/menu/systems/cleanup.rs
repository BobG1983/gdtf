use bevy::prelude::*;

use crate::scenes::running::menu::resources::MenuComplete;

pub(in crate::scenes::running::menu) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<MenuComplete>();
}
