use bevy::prelude::*;

use crate::scenes::load::resources::LoadComplete;

pub(in crate::scenes::load) fn load_complete(mut commands: Commands) {
    commands.insert_resource(LoadComplete);
}
