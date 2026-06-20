use bevy::prelude::*;

use crate::states::AfterMathState;

pub(in crate::states::running::game::battlescape::aftermath::display_aftermath) fn move_on(
    mut state: ResMut<NextState<AfterMathState>>,
) {
    state.set(AfterMathState::AnimateOut);
}
