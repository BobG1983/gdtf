//! The [`MapEditorApp`] wrapper — the editor binary's entry point.
//!
//! Mirrors `gdtf_app`'s `GdtfApp`: a thin newtype over a Bevy [`App`] that composes
//! `DefaultPlugins` (with the asset source root pointed at the workspace `assets/`), the
//! [`EguiPlugin`](bevy_egui::EguiPlugin) (GTW-512: the editor's UI is now egui — a CLEAN SWAP off
//! the game's hand-rolled UI plugin), the editor's own [`MapEditorPlugin`], and the env-gated
//! QA capture affordance — plus, under the GTW-804 `debug_assertions` + `net_qa` double gate, the
//! env-gated DEV QA network control channel (`NetQaEditorPlugin`). It is a SEPARATE binary from the
//! game — it shares no scene graph and runs
//! no battle sim (the GTW-417 housing constraint).

use bevy::{asset::AssetPlugin, prelude::*};
use bevy_egui::EguiPlugin;
// GTW-634 C1: the workspace `assets/` root has ONE owning definition — the shared
// `gdtf_assets` constant both hosts' `AssetPlugin` AND every editor-side saver import,
// so what the editor loads-from and saves-into can never drift from the game.
use gdtf_assets::WORKSPACE_ASSETS_ROOT;

use crate::{capture::EditorCapturePlugin, plugin::MapEditorPlugin};

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
        // GTW-804: the DEV-ONLY QA network control channel, under its DOUBLE gate — a debug
        // build AND the opt-in `net_qa` feature, because it opens a loopback TCP listener, so a
        // release editor (or a default-feature one) never compiles it in. Even then it is inert
        // until `GDTF_EDITOR_NET_QA` is set truthy: the env read happens in `from_env`, and an
        // unset variable registers nothing at all. Written as an attribute on the call rather
        // than a helper fn so the feature-off build has no empty fn for clippy to flag.
        #[cfg(all(debug_assertions, feature = "net_qa"))]
        app.add_plugins(crate::net_qa::NetQaEditorPlugin::from_env());
        Self(app)
    }

    /// Run the map-editor application.
    pub fn run(mut self) {
        self.0.run();
    }
}

/// Wire the GTW-510 interactive F10 screenshot keybind, DEV-only.
///
/// Gated on `cfg!(debug_assertions)`, so a release editor never compiles it in. On F10 the
/// `gdtf_screenshot` crate captures the primary window to a timestamped
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
