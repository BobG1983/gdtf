//! Bevy app wrapper for the GDTF game binary.

use bevy::{asset::AssetPlugin, prelude::*};
use cobalt_ron_assets::workspace_assets_root;
use gdtf_ui::UiPlugin;

use crate::states::{AppState, ScenesPlugin};

/// Top-level application handle.
pub struct GdtfApp(App);

impl GdtfApp {
    /// Build a new app with default plugins and scenes.
    #[must_use]
    pub fn new() -> Self {
        let app = Self(App::new());

        app.add_bevy_plugins().add_states().add_plugins()
    }

    /// Run the app until exit.
    pub fn run(mut self) {
        self.0.run();
    }

    #[must_use]
    fn add_bevy_plugins(mut self) -> Self {
        let assets = workspace_assets_root();
        self.0.add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: assets.as_ref().map_or_else(
                || AssetPlugin::default().file_path,
                |root| root.to_string_lossy().into_owned(),
            ),
            ..default()
        }));
        if assets.is_none() {
            error!(
                "found no `Cargo.lock` or `[workspace]` manifest above the crate — keeping Bevy's \
                 default asset path"
            );
        }
        self
    }

    #[must_use]
    fn add_plugins(mut self) -> Self {
        self.0.add_plugins(ScenesPlugin);
        self.0.add_plugins(UiPlugin);
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
