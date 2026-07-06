//! Core wiring and app logic for GDTF.

use bevy::{asset::AssetPlugin, prelude::*};
// GTW-634 C1: the workspace `assets/` root has ONE owning definition — the shared
// `gdtf_assets` constant (see its doc for the `get_base_path` rationale) that both
// hosts' `AssetPlugin` AND every editor-side saver import, so a saved file lands
// exactly where this app loads from, by construction.
use gdtf_assets::WORKSPACE_ASSETS_ROOT;
use gdtf_ui::UiPlugin;

use crate::states::{AppState, ScenesPlugin};

/// Main entry point for the GDTF application.
pub struct GdtfApp(App);

impl GdtfApp {
    /// Crate a new GDTF application instance.
    #[must_use]
    pub fn new() -> Self {
        let app = Self(App::new());

        app.add_bevy_plugins().add_states().add_plugins()
    }

    /// Run the GDTF application.
    pub fn run(mut self) {
        self.0.run();
    }

    #[must_use]
    fn add_bevy_plugins(mut self) -> Self {
        // This default `AssetPlugin` construction is intentionally byte-identical
        // regardless of features: with `watch_for_changes_override: None` (the
        // default), Bevy starts its asset file watcher iff its internal `watch`
        // cfg is set. The dev-only `file_watcher` feature (gdtf_app ->
        // bevy/file_watcher) sets that cfg transitively, turning this source root
        // into a hot-reload watcher with NO code change here. See the
        // `file_watcher` feature comment in this crate's Cargo.toml.
        self.0.add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: WORKSPACE_ASSETS_ROOT.to_owned(),
            ..default()
        }));
        self
    }

    #[must_use]
    fn add_plugins(mut self) -> Self {
        self.0.add_plugins(ScenesPlugin);
        self.0.add_plugins(UiPlugin);
        // The DEV-ONLY QA affordances (auto-battle, capture, the drive triggers, the
        // F10 screenshot keybind) aggregate under the one `crate::dev` plugin owner;
        // every cfg/env gate lives inside it, so this add registers nothing in a
        // release artifact (GTW-632).
        self.0.add_plugins(crate::dev::DevAffordancesPlugin);
        self
    }

    #[must_use]
    fn add_states(mut self) -> Self {
        self.0.init_state::<AppState>();
        self
    }
}

impl Default for GdtfApp {
    fn default() -> Self {
        Self::new()
    }
}
