use bevy::prelude::*;

use crate::states::GameState;

pub(in crate::states::running::game::hivescape) fn move_on(
    mut state: ResMut<NextState<GameState>>,
) {
    state.set(GameState::BattleScape);
}
