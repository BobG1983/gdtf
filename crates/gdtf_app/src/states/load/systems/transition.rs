use bevy::prelude::*;

use crate::states::AppState;

pub(in crate::states::load) fn transition_to_intro(next: Option<ResMut<NextState<AppState>>>) {
    if let Some(mut next) = next {
        next.set(AppState::Intro);
    }
}
