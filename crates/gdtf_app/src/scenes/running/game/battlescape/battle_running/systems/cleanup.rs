use bevy::prelude::*;

use crate::scenes::running::game::battlescape::battle_running::resources::BattleRunningComplete;

pub(in crate::scenes::running::game::battlescape::battle_running) fn cleanup(
    mut commands: Commands,
) {
    commands.remove_resource::<BattleRunningComplete>();
}
