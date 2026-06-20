use bevy::prelude::*;

use crate::states::AppState;

pub(in crate::states::running::quit) fn move_on(mut state: ResMut<NextState<AppState>>) {
    state.set(AppState::Teardown);
}
