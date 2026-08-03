use bevy::{asset::AssetPlugin, prelude::*};
use bevy_egui::EguiPlugin;
use gdtf_assets::WORKSPACE_ASSETS_ROOT;

use crate::{capture::EditorCapturePlugin, plugin::MapEditorPlugin};

pub struct MapEditorApp(App);

impl MapEditorApp {
        #[must_use]
    pub fn new() -> Self {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: WORKSPACE_ASSETS_ROOT.to_owned(),
            ..default()
        }));
        app.add_plugins(EguiPlugin::default());
        app.add_plugins(MapEditorPlugin);
        app.add_plugins(EditorCapturePlugin::from_env());
        add_dev_keybind(&mut app);
        #[cfg(all(debug_assertions, feature = "net_qa"))]
        app.add_plugins(crate::net_qa::NetQaEditorPlugin::from_env());
        Self(app)
    }

        pub fn run(mut self) {
        self.0.run();
    }
}

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
