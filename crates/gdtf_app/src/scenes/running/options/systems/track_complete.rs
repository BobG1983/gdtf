use bevy::prelude::*;

use crate::scenes::running::options::resources::OptionsComplete;

pub(in crate::scenes::running::options) fn options_complete(mut commands: Commands) {
    commands.insert_resource(OptionsComplete);
}
