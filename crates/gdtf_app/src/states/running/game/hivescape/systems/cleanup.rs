use bevy::prelude::*;

use crate::states::running::game::hivescape::resources::HiveScapeComplete;

pub(in crate::states::running::game::hivescape) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<HiveScapeComplete>();
}
