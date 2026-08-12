//! Test app with `DefaultPlugins` and scene registration for load tests.

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
    let Some(root) = gdtf_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

/// Builds a headless `DefaultPlugins` app with scenes registered.
pub struct GdtfLoadTestAppBuilder {
    app: App,
}

impl GdtfLoadTestAppBuilder {
    /// Use the workspace assets directory.
    #[must_use]
    pub fn new() -> Self {
        Self::with_asset_root(workspace_assets_root())
    }

    /// Use an explicit asset root path.
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

    /// Queue a transition into `state` before the first update.
    #[must_use]
    pub fn starting_in(mut self, state: AppState) -> Self {
        self.app
            .world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(state);
        self
    }

    /// Finish building and return the app.
    pub fn build(self) -> App {
        self.app
    }
}

impl Default for GdtfLoadTestAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}
