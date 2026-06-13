use bevy::prelude::*;

use crate::states::AppState;

pub(in crate::scenes::intro) fn move_on(mut state: ResMut<NextState<AppState>>) {
    state.set(AppState::Running);
}
