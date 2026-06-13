use bevy::prelude::*;

use crate::scenes::init::resources::InitComplete;

pub(in crate::scenes::init) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<InitComplete>();
}
