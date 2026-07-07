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
        /// DEV-ONLY procgen STEP/AUTO visualizer (GTW-434) — a debug overlay that
        /// reveals the procgen space-packing placement sequence one prefab at a time
        /// (STEP) or all at once (AUTO), drawn as tinted quads over a dark board quad.
        ///
        /// Reached **only** from the `cfg(debug_assertions)`-gated "Procgen Viz"
        /// main-menu button (so the entry point never compiles into a release binary),
        /// and torn down on `OnExit` — both the screen entities and the visualizer
        /// model resource. The entire visualizer module (`procgen_viz` and its scene
        /// plugin) is itself `cfg(debug_assertions)`-gated, so the feature compiles out
        /// of release (C4); the variant remains (a `SubStates` enum cannot easily
        /// cfg-gate one discriminant) but with no non-debug code transitioning to it, it
        /// is unreachable in release.
        DebugProcgenVisualizer,
    }
}
