use bevy::prelude::*;

use crate::states::RunningState;

pub(in crate::states::running::options) fn move_on(mut state: ResMut<NextState<RunningState>>) {
    state.set(RunningState::Game);
}
