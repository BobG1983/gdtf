use bevy::prelude::*;

use crate::states::AppState;

#[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[source(AppState = AppState::Running)]
pub(crate) enum RunningState {
    #[default]
    Menu,
    Game,
    Options,
    Quit,
}
