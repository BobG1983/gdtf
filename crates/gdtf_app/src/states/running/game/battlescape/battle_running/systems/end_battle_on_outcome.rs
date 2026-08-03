use bevy::prelude::*;

use crate::states::running::game::battlescape::battle_running::resources::BattleRunningComplete;

pub(in crate::states::running::game::battlescape::battle_running) fn end_battle_on_outcome(
    mut won: MessageReader<gdtf_battle_sim::battle::BattleWon>,
    mut lost: MessageReader<gdtf_battle_sim::battle::BattleLost>,
    mut commands: Commands,
) {
    let decided = won.read().next().is_some() | lost.read().next().is_some();
    if decided {
        commands.insert_resource(BattleRunningComplete);
    }
}
