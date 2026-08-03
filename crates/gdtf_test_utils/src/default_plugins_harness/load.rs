use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::error::warn,
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    state::state::NextState,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_app::test_support::{self, AppState};

fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

pub struct GdtfLoadTestAppBuilder {
        app: App,
}

impl GdtfLoadTestAppBuilder {
            #[must_use]
    pub fn new() -> Self {
        Self::with_asset_root(workspace_assets_root())
    }

                                    #[must_use]
    pub fn with_asset_root(root: PathBuf) -> Self {
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .set(RenderPlugin {
                    render_creation: WgpuSettings {
                        backends: None,
                        ..default()
                    }
                    .into(),
                    ..default()
                })
                .disable::<WinitPlugin>()
                .disable::<bevy::log::LogPlugin>()
                .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
                .disable::<bevy::gizmos::GizmoPlugin>()
                .disable::<bevy::audio::AudioPlugin>()
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: root.to_string_lossy().into_owned(),
                    ..default()
                }),
        );
        app.set_error_handler(warn);
        test_support::register_scenes_with_default_plugins(&mut app);
        Self { app }
    }

                    #[must_use]
    pub fn starting_in(mut self, state: AppState) -> Self {
        self.app
            .world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(state);
        self
    }

            pub fn build(self) -> App {
        self.app
    }
}

impl Default for GdtfLoadTestAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}
