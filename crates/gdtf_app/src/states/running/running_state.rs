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
        /// DEV-ONLY in-app gang editor (GTW-420), the foundation of the GTW-403
        /// gang-editor track.
        ///
        /// Reached **only** from the `cfg(debug_assertions)`-gated "Gang Editor"
        /// main-menu button (so the entry point never compiles into a release
        /// binary), and torn down on `OnExit` — both the screen entities and the
        /// editor model resource. The variant itself is always present (a
        /// `SubStates` enum cannot easily cfg-gate one discriminant), but with no
        /// non-debug code transitioning to it, it is unreachable in release.
        DebugEditor,
    }
}
