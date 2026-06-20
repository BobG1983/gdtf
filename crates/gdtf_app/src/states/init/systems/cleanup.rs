use bevy::prelude::*;

use crate::states::init::resources::InitComplete;

pub(in crate::states::init) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<InitComplete>();
}
