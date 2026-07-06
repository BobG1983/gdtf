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
        add_dev_affordances(&mut self.0);
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

/// Wires DEV-ONLY affordances that must never reach a release artifact.
///
/// Gated on `cfg!(debug_assertions)` so a release build does not even compile the
/// auto-enter-battle affordance in (`crate::app::auto_battle`). The affordance is
/// *additionally* inert by default at runtime — it activates only when
/// `GDTF_AUTOBATTLE` is set truthy
/// ([`auto_battle_enabled`](crate::app::auto_battle::auto_battle_enabled)) — so a
/// normal `cargo run` (debug) still reaches the menu and stops. The `cfg` keeps the
/// release binary clean; the env gate keeps the dev launch inert until opted into.
///
/// The GTW-297 screenshot / visual-QA affordance (`crate::app::capture`) is wired in
/// beside it, DOUBLE-gated on `cfg!(all(debug_assertions, feature = "dev_capture"))` —
/// dev feature AND debug build — so it never reaches a release artifact (release builds
/// enable neither). Since GTW-590 the binary's `dynamic_linking` dev feature folds
/// `dev_capture` in, so every dynamic-linked dev/gate build compiles it. It too is
/// inert at runtime until `GDTF_CAPTURE_PATH` is set. Paired with the auto-battle
/// affordance, it captures a live battlescape frame to a PNG unattended (see
/// `crate::app::capture` for the invocation).
///
/// Takes `&mut App` (the ordinary Bevy app-builder handle, like
/// [`App::add_plugins`]); this is NOT a registered system or a `&mut World` helper,
/// so `bevy-traps.md` #7 does not apply.
fn add_dev_affordances(app: &mut App) {
    #[cfg(debug_assertions)]
    app.add_plugins(crate::app::auto_battle::AutoBattlePlugin::from_env());
    #[cfg(all(debug_assertions, feature = "dev_capture"))]
    app.add_plugins(crate::app::capture::DevCapturePlugin::from_env());
    // GTW-510: the interactive F10 screenshot keybind (captures the primary window to a
    // timestamped `target/screenshots/game-<secs>.png` without exiting). Double-gated on
    // `dev_capture` + debug — the reusable `gdtf_screenshot` crate is pulled in only by the
    // `dev_capture` feature, so a release binary never links it.
    #[cfg(all(debug_assertions, feature = "dev_capture"))]
    app.add_plugins(gdtf_screenshot::KeyboardCapturePlugin::new("game"));
    #[cfg(not(debug_assertions))]
    let _ = app;
}
