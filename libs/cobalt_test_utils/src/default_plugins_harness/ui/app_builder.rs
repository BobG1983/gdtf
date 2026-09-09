//! Typed-builder phases for a headless UI test app.

use core::marker::PhantomData;
use std::path::{Path, PathBuf};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    camera::Camera2d,
    ecs::error::warn,
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};

use crate::asset_plugin::asset_plugin_at;

/// Builder phase: no camera spawned yet.
pub struct NoCamera;

/// Builder phase: a `Camera2d` has been spawned.
pub struct WithCamera;

/// Headless `DefaultPlugins` app for UI tests.
pub struct UiTestAppBuilder<Phase> {
    app:    App,
    _phase: PhantomData<fn() -> Phase>,
}

fn workspace_assets_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

impl UiTestAppBuilder<NoCamera> {
    /// Start a headless `DefaultPlugins` app reading the workspace assets (no camera).
    #[must_use]
    pub fn new() -> Self {
        Self::on_asset_root(&workspace_assets_root())
    }

    /// Start a headless `DefaultPlugins` app reading assets from `root` (no camera).
    #[must_use]
    pub fn on_asset_root(root: &Path) -> Self {
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
                .set(asset_plugin_at(root)),
        );
        app.set_error_handler(warn);
        Self {
            app,
            _phase: PhantomData,
        }
    }

    /// Spawn a `Camera2d` and move to the `WithCamera` phase.
    #[must_use]
    pub fn with_ui_camera(mut self) -> UiTestAppBuilder<WithCamera> {
        self.app.world_mut().spawn(Camera2d);
        UiTestAppBuilder {
            app:    self.app,
            _phase: PhantomData,
        }
    }
}

impl Default for UiTestAppBuilder<NoCamera> {
    fn default() -> Self {
        Self::new()
    }
}

impl UiTestAppBuilder<WithCamera> {
    /// Finish building and return the app.
    pub fn build(self) -> App {
        self.app
    }
}
