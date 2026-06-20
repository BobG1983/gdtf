use bevy::prelude::*;

use crate::states::teardown::resources::TeardownComplete;

pub(in crate::states::teardown) fn teardown_complete(mut commands: Commands) {
    commands.insert_resource(TeardownComplete);
}
