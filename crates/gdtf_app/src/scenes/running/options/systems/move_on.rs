use bevy::prelude::*;

use crate::states::RunningState;

pub(in crate::scenes::running::options) fn move_on(mut state: ResMut<NextState<RunningState>>) {
    state.set(RunningState::Game);
}
