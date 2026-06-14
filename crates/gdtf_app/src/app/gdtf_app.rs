//! Core wiring and app logic for GDTF.

use bevy::{asset::AssetPlugin, prelude::*};
use gdtf_ui::UiPlugin;

use crate::{scenes::ScenesPlugin, states::AppState};

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
