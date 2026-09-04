use bevy::prelude::*;

use crate::states::load::resources::{LoadFailed, LoadHandles};

pub(in crate::states::load) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<LoadHandles>();
    commands.remove_resource::<LoadFailed>();
}
