use bevy::prelude::*;

use crate::states::AppState;

pub(in crate::scenes::load) fn move_on(mut state: ResMut<NextState<AppState>>) {
    state.set(AppState::Intro);
}
