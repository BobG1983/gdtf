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

pub struct NoCamera;

pub struct WithCamera;

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
                        pub fn build(self) -> App {
        self.app
    }
}
