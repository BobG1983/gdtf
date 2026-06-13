use bevy::prelude::*;

use crate::states::GameState;

pub(in crate::scenes::running::game::setup) fn move_on(mut state: ResMut<NextState<GameState>>) {
    state.set(GameState::HiveScape);
}
