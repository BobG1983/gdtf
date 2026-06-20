use bevy::prelude::*;

use crate::states::init::resources::InitComplete;

pub(in crate::states::init) fn init_complete(mut commands: Commands) {
    commands.insert_resource(InitComplete);
}
