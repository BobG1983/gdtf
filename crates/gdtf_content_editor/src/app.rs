//! The [`MapEditorApp`] wrapper — the editor binary's entry point.
//!
//! Mirrors `gdtf_app`'s `GdtfApp`: a thin newtype over a Bevy [`App`] that composes
//! `DefaultPlugins` (with the asset source root pointed at the workspace `assets/`), the
//! `gdtf_ui` [`UiPlugin`](gdtf_ui::UiPlugin), the editor's own [`MapEditorPlugin`], and the
//! env-gated QA capture affordance. It is a SEPARATE binary from the game — it shares no
//! scene graph and runs no battle sim (the GTW-417 housing constraint).

use bevy::{asset::AssetPlugin, prelude::*};
use gdtf_ui::UiPlugin;

use crate::{capture::EditorCapturePlugin, plugin::MapEditorPlugin};

/// Absolute path to the workspace-root `assets/` directory.
///
/// Identical rationale to `gdtf_app`'s `GdtfApp`: Bevy's default file asset reader resolves
/// its base from `BEVY_ASSET_ROOT`, else `CARGO_MANIFEST_DIR`, else the executable dir.
/// Under `cargo run -p gdtf_content_editor_bin`, `CARGO_MANIFEST_DIR` is the `bins/gdtf_content_editor` package,
/// so the default would look under `bins/gdtf_content_editor/assets`. We therefore point
/// [`AssetPlugin::file_path`] at the workspace root, computed at compile time relative to
/// THIS crate's manifest (`crates/gdtf_content_editor` → up two levels → `assets`). This matches
/// both `gdtf_app` and the headless test harness, so a path that loads in one loads in all.
const WORKSPACE_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

/// The GDTF map-editor application — a SEPARATE windowed binary from the game.
pub struct MapEditorApp(App);

impl MapEditorApp {
    /// Create a new map-editor application instance, fully wired.
    #[must_use]
    pub fn new() -> Self {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: WORKSPACE_ASSETS_ROOT.to_owned(),
            ..default()
        }));
        app.add_plugins(UiPlugin);
        app.add_plugins(MapEditorPlugin);
        // QA / debug-only screenshot-then-exit (AC4). Inert by default — wires nothing
        // unless GDTF_EDITOR_SHOT is set (the env read happens in `from_env`).
        app.add_plugins(EditorCapturePlugin::from_env());
        Self(app)
    }

    /// Run the map-editor application.
    pub fn run(mut self) {
        self.0.run();
    }
}

impl Default for MapEditorApp {
    fn default() -> Self {
        Self::new()
    }
}
