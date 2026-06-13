use bevy::prelude::*;

use crate::states::AfterMathState;

pub(in crate::scenes::running::game::battlescape::aftermath::display_after) fn move_on(
    mut state: ResMut<NextState<AfterMathState>>,
) {
    state.set(AfterMathState::AnimateOut);
}
