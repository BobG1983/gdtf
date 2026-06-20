use bevy::prelude::*;

use crate::states::teardown::resources::TeardownComplete;

pub(in crate::states::teardown) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<TeardownComplete>();
}
