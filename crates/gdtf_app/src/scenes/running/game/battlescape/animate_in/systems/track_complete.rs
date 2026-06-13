use bevy::prelude::*;

use crate::scenes::running::game::battlescape::animate_in::resources::BattleAnimateInComplete;

pub(in crate::scenes::running::game::battlescape::animate_in) fn game_battlescape_animate_in_complete(
    mut commands: Commands,
) {
    commands.insert_resource(BattleAnimateInComplete);
}
