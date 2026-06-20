use bevy::prelude::*;

use crate::states::AfterMathState;

pub(in crate::states::running::game::battlescape::aftermath::animate_in) fn move_on(
    mut state: ResMut<NextState<AfterMathState>>,
) {
    state.set(AfterMathState::DisplayAftermath);
}
