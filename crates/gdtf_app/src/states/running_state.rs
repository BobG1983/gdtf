use bevy::prelude::*;

use crate::states::AppState;

crate::support_item! {
    /// Sub-state of [`AppState::Running`]: which top-level running screen is active.
    #[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[source(AppState = AppState::Running)]
    enum RunningState {
        /// Main menu.
        #[default]
        Menu,
        /// In-game (setup, hive layer, battle layer).
        Game,
        /// Options screen.
        Options,
        /// Quit confirmation / shutdown.
        Quit,
    }
}
