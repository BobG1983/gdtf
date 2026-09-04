//! Top-level application state machine.

use bevy::prelude::*;

crate::support_item! {
    /// Top-level application lifecycle.
    #[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
    enum AppState {
        /// Startup bootstrap.
        #[default]
        Init,
        /// Loading assets and tables.
        Load,
        /// Intro / splash.
        Intro,
        /// Main interactive loop.
        Running,
        /// Shutdown cleanup.
        Teardown,
    }
}
