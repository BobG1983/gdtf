use bevy::prelude::*;

use crate::scenes::load::resources::LoadComplete;

pub(in crate::scenes::load) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<LoadComplete>();
}
