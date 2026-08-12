//! Map editor application entry wrapper.

use bevy::{asset::AssetPlugin, prelude::*};
use bevy_egui::EguiPlugin;
use gdtf_assets::workspace_assets_root;

use crate::plugin::MapEditorPlugin;

/// Owned Bevy app configured for the content editor.
pub struct MapEditorApp(App);

impl MapEditorApp {
    /// Build a new editor app with default plugins and assets root.
    #[must_use]
    pub fn new() -> Self {
        let mut app = App::new();
        let assets = workspace_assets_root();
        app.add_plugins(DefaultPlugins.set(AssetPlugin {
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
        app.add_plugins(EguiPlugin::default());
        app.add_plugins(MapEditorPlugin);
        #[cfg(debug_assertions)]
        app.add_plugins(crate::net_qa::NetQaEditorPlugin::from_env());
        Self(app)
    }

    /// Run the app until exit.
    pub fn run(mut self) {
        self.0.run();
    }
}

impl Default for MapEditorApp {
    fn default() -> Self {
        Self::new()
    }
}
