use bevy::prelude::*;

use crate::scenes::running::game::battlescape::animate_out::resources::BattleAnimateOutComplete;

pub(in crate::scenes::running::game::battlescape::animate_out) fn game_battlescape_animate_out_complete(
    mut commands: Commands,
) {
    commands.insert_resource(BattleAnimateOutComplete);
}
