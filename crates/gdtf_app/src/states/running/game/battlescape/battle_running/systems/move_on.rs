use bevy::prelude::*;

use crate::states::BattleScapeState;

pub(in crate::states::running::game::battlescape::battle_running) fn move_on(
    mut state: ResMut<NextState<BattleScapeState>>,
) {
    state.set(BattleScapeState::AnimateOut);
}
