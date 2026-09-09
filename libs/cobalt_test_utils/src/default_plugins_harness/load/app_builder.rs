//! Test app with `DefaultPlugins` and caller-supplied scene registration for load tests.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    ecs::error::warn,
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    state::state::{FreelyMutableState, NextState, States},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};

use crate::asset_plugin::asset_plugin_at;

fn workspace_assets_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

/// Builds a headless `DefaultPlugins` app with the caller's scenes registered.
pub struct LoadTestAppBuilder {
    app: App,
}

impl LoadTestAppBuilder {
    /// Use the workspace assets directory, then `register`.
    #[must_use]
    pub fn new(register: impl FnOnce(&mut App)) -> Self {
        Self::with_asset_root(workspace_assets_root(), register)
    }

    /// Use an explicit asset root path, then `register`.
    #[must_use]
    pub fn with_asset_root(root: PathBuf, register: impl FnOnce(&mut App)) -> Self {
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
                .set(asset_plugin_at(&root)),
        );
        app.set_error_handler(warn);
        register(&mut app);
        Self { app }
    }

    /// Queue a transition into `state` before the first update.
    #[must_use]
    pub fn starting_in<S: States + FreelyMutableState>(mut self, state: S) -> Self {
        self.app
            .world_mut()
            .resource_mut::<NextState<S>>()
            .set(state);
        self
    }

    /// Finish building and return the app.
    pub fn build(self) -> App {
        self.app
    }
}
