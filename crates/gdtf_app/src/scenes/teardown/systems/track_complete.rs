use bevy::prelude::*;

use crate::scenes::teardown::resources::TeardownComplete;

pub(in crate::scenes::teardown) fn teardown_complete(mut commands: Commands) {
    commands.insert_resource(TeardownComplete);
}
