//! The headless editor app every content suite drives, on an assets root it names.

use std::path::Path;

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use cobalt_test_utils::asset_plugin_at;
use gdtf_editor::MapEditorPlugin;

/// A headless editor app whose asset server reads `root` instead of the workspace assets.
pub(crate) fn editor_app_with_asset_root(root: &Path) -> App {
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
    app.add_plugins(MapEditorPlugin);
    app
}
