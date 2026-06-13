use bevy::prelude::*;

use crate::scenes::running::game::hivescape::resources::HiveScapeComplete;

pub(in crate::scenes::running::game::hivescape) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<HiveScapeComplete>();
}
