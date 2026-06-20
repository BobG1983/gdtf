use bevy::prelude::*;

use crate::states::running::options::resources::OptionsComplete;

pub(in crate::states::running::options) fn options_complete(mut commands: Commands) {
    commands.insert_resource(OptionsComplete);
}
