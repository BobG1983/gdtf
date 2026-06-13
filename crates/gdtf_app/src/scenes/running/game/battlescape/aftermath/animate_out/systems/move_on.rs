use bevy::prelude::*;

use crate::states::RunningState;

pub(in crate::scenes::running::game::battlescape::aftermath::animate_out) fn move_on(
    mut state: ResMut<NextState<RunningState>>,
) {
    // Terminal of the deepest sub-machine. AfterMath, BattleScape, and Game are each
    // the last state at their level, so finishing here pops all the way out of Game to
    // RunningState::Quit, which in turn advances AppState to Teardown.
    state.set(RunningState::Quit);
}
