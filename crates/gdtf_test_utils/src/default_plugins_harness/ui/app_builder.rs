//! Typed-builder phases for a headless UI test app.

use core::marker::PhantomData;
use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    camera::Camera2d,
    ecs::error::warn,
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};

/// Builder phase: no camera spawned yet.
pub struct NoCamera;

/// Builder phase: a `Camera2d` has been spawned.
pub struct WithCamera;

/// Headless `DefaultPlugins` app for UI tests.
pub struct GdtfUiTestAppBuilder<Phase> {
    app:    App,
    _phase: PhantomData<fn() -> Phase>,
}

fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

impl GdtfUiTestAppBuilder<NoCamera> {
    /// Start a headless `DefaultPlugins` app (no camera).
    #[must_use]
    pub fn new() -> Self {
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
                    file_path: workspace_assets_root().to_string_lossy().into_owned(),
                    ..default()
                }),
        );
        app.set_error_handler(warn);
        Self {
            app,
            _phase: PhantomData,
        }
    }

    /// Spawn a `Camera2d` and move to the `WithCamera` phase.
    #[must_use]
    pub fn with_ui_camera(mut self) -> GdtfUiTestAppBuilder<WithCamera> {
        self.app.world_mut().spawn(Camera2d);
        GdtfUiTestAppBuilder {
            app:    self.app,
            _phase: PhantomData,
        }
    }
}

impl Default for GdtfUiTestAppBuilder<NoCamera> {
    fn default() -> Self {
        Self::new()
    }
}

impl GdtfUiTestAppBuilder<WithCamera> {
    /// Finish building and return the app.
    pub fn build(self) -> App {
        self.app
    }
}
