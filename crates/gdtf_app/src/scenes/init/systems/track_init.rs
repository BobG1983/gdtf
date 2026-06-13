use bevy::prelude::*;

use crate::scenes::init::resources::InitComplete;

pub(in crate::scenes::init) fn init_complete(mut commands: Commands) {
    commands.insert_resource(InitComplete);
}
