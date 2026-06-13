use bevy::prelude::*;

use crate::scenes::running::options::resources::OptionsComplete;

pub(in crate::scenes::running::options) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<OptionsComplete>();
}
