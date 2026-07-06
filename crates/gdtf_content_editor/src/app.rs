//! The [`MapEditorApp`] wrapper — the editor binary's entry point.
//!
//! Mirrors `gdtf_app`'s `GdtfApp`: a thin newtype over a Bevy [`App`] that composes
//! `DefaultPlugins` (with the asset source root pointed at the workspace `assets/`), the
//! [`EguiPlugin`](bevy_egui::EguiPlugin) (GTW-512: the editor's UI is now egui — a CLEAN SWAP off
//! the game's hand-rolled UI plugin), the editor's own [`MapEditorPlugin`], and the env-gated
//! QA capture affordance. It is a SEPARATE binary from the game — it shares no scene graph and runs
//! no battle sim (the GTW-417 housing constraint).

use bevy::{asset::AssetPlugin, prelude::*};
use bevy_egui::EguiPlugin;

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
        // GTW-512 C1.1: the egui plugin. `EguiPlugin::default()` is the recommended multipass-aware
        // wiring (the option to disable multipass is deprecated); the editor's UI systems live in
        // the `EguiPrimaryContextPass` schedule so they work under both single- and multi-pass.
        app.add_plugins(EguiPlugin::default());
        app.add_plugins(MapEditorPlugin);
        // QA / debug-only screenshot-then-exit (AC4). Inert by default — wires nothing
        // unless GDTF_EDITOR_SHOT is set (the env read happens in `from_env`).
        app.add_plugins(EditorCapturePlugin::from_env());
        add_dev_keybind(&mut app);
        Self(app)
    }

    /// Run the map-editor application.
    pub fn run(mut self) {
        self.0.run();
    }
}

/// Wire the GTW-510 interactive F10 screenshot keybind, DEV-only.
///
/// Gated on `cfg!(debug_assertions)` (the procgen-viz keybind precedent), so a release editor never
/// compiles it in. On F10 the `gdtf_screenshot` crate captures the primary window to a timestamped
/// PNG under `target/screenshots/editor-<secs>.png` WITHOUT exiting — an interactive dev capture,
/// distinct from the env-gated `EditorCapturePlugin` capture-then-exit QA path.
///
/// Takes `&mut App` (the ordinary app-builder handle, like [`App::add_plugins`]); not a registered
/// system or a `&mut World` helper, so `bevy-traps.md` #7 does not apply.
fn add_dev_keybind(app: &mut App) {
    #[cfg(debug_assertions)]
    app.add_plugins(gdtf_screenshot::KeyboardCapturePlugin::new("editor"));
    #[cfg(not(debug_assertions))]
    let _ = app;
}

impl Default for MapEditorApp {
    fn default() -> Self {
        Self::new()
    }
}
