//! Core wiring and app logic for GDTF.

use bevy::{asset::AssetPlugin, prelude::*};
use gdtf_ui::UiPlugin;

use crate::states::{AppState, ScenesPlugin};

/// Absolute path to the workspace-root `assets/` directory.
///
/// GDTF ships its loose `.ron`/font assets under the **repo-root** `assets/`
/// (ADR 0003), and the [`AssetServer`] must resolve `assets/...` paths against
/// that directory. Bevy's default file
/// [`AssetReader`](bevy::asset::io::AssetReader) base path is **not** the
/// working directory — `bevy_asset`'s `get_base_path` reads `BEVY_ASSET_ROOT`,
/// else the runtime `CARGO_MANIFEST_DIR`, else the executable's directory. Under
/// `cargo run -p grimdark_turfwar`, cargo sets `CARGO_MANIFEST_DIR` in the child
/// process to the **binary** package (`bins/grimdark_turfwar`), so the default
/// would look under `bins/grimdark_turfwar/assets` — not the repo root. We
/// therefore point [`AssetPlugin::file_path`] explicitly at the workspace root,
/// computed at compile time relative to **this** crate's manifest
/// (`crates/gdtf_app` → up two levels → `assets`). This mirrors the headless
/// test harness (`GdtfUiTestAppBuilder`), so a path that loads in a test loads
/// in the app.
const WORKSPACE_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

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
/// opt-in feature AND debug build — so it never reaches a release artifact and is not
/// compiled by the default suite. It too is inert at runtime until `GDTF_CAPTURE_PATH`
/// is set. Paired with the auto-battle affordance, it captures a live battlescape frame
/// to a PNG unattended (see `crate::app::capture` for the invocation).
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
