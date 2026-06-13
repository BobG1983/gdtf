use bevy::prelude::*;

use crate::scenes::running::game::battlescape::animate_out::resources::BattleAnimateOutComplete;

pub(in crate::scenes::running::game::battlescape::animate_out) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<BattleAnimateOutComplete>();
}
