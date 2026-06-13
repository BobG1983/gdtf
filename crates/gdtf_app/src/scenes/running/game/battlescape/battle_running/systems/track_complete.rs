use bevy::prelude::*;

use crate::scenes::running::game::battlescape::battle_running::resources::BattleRunningComplete;

pub(in crate::scenes::running::game::battlescape::battle_running) fn game_battlescape_battle_running_complete(
    mut commands: Commands,
) {
    commands.insert_resource(BattleRunningComplete);
}
