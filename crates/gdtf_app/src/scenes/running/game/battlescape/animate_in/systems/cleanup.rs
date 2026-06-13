use bevy::prelude::*;

use crate::scenes::running::game::battlescape::animate_in::resources::BattleAnimateInComplete;

pub(in crate::scenes::running::game::battlescape::animate_in) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<BattleAnimateInComplete>();
}
