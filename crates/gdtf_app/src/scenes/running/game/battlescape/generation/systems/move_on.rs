use bevy::prelude::*;

use crate::states::BattleScapeState;

pub(in crate::scenes::running::game::battlescape::generation) fn move_on(
    mut state: ResMut<NextState<BattleScapeState>>,
) {
    state.set(BattleScapeState::AnimateIn);
}
