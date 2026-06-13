use bevy::prelude::*;

crate::support_item! {
    /// Top-level application lifecycle state.
    #[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
    enum AppState {
        /// One-shot bootstrap before any scene runs.
        #[default]
        Init,
        /// Asset / scene loading hand-off.
        Load,
        /// Intro / splash scene.
        Intro,
        /// The running game (menu, game, options, quit live under this).
        Running,
        /// Final teardown before exit.
        Teardown,
    }
}
