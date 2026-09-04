//! Sub-states while the app is in [`AppState::Running`].

use bevy::prelude::*;

use crate::states::AppState;

crate::support_item! {
    /// Modes available under Running.
    #[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[source(AppState = AppState::Running)]
    enum RunningState {
        /// Main menu.
        #[default]
        Menu,
        /// In a game session.
        Game,
        /// Options screen.
        Options,
        /// Quit requested.
        Quit,
    }
}
