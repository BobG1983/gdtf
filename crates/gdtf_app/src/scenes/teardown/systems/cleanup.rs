use bevy::prelude::*;

use crate::scenes::teardown::resources::TeardownComplete;

pub(in crate::scenes::teardown) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<TeardownComplete>();
}
