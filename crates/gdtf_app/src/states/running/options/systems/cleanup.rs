use bevy::prelude::*;

use crate::states::running::options::resources::OptionsComplete;

pub(in crate::states::running::options) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<OptionsComplete>();
}
