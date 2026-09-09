//! Headless app that still owns a primary window, for render-target tests.

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    ecs::error::warn,
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, Window, WindowPlugin},
    winit::WinitPlugin,
};

use crate::asset_plugin::unwatched_asset_plugin;

/// Headless `DefaultPlugins` app with a primary window and no wgpu backend.
///
/// Window-sized render targets and scale factors are real; nothing is drawn.
pub struct WindowedTestAppBuilder {
    window: Window,
}

impl WindowedTestAppBuilder {
    /// Start with Bevy's default primary window.
    #[must_use]
    pub fn new() -> Self {
        Self {
            window: Window::default(),
        }
    }

    /// Pin the primary window's scale factor.
    #[must_use]
    pub fn scale_factor(mut self, scale_factor: f32) -> Self {
        self.window.resolution.set_scale_factor(scale_factor);
        self
    }

    /// Finish building and return the app.
    pub fn build(self) -> App {
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
                    primary_window: Some(self.window),
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .set(unwatched_asset_plugin()),
        );
        app.set_error_handler(warn);
        app
    }
}

impl Default for WindowedTestAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}
