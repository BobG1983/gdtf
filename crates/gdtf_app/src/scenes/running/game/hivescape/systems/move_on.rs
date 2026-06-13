use bevy::prelude::*;

use crate::states::GameState;

pub(in crate::scenes::running::game::hivescape) fn move_on(
    mut state: ResMut<NextState<GameState>>,
) {
    state.set(GameState::BattleScape);
}
