//! handful of sim resources, so this fixture authors those DIRECTLY via
use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    ecs::error::warn,
    prelude::{Text2d, Visibility, With, default},
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use cobalt_test_utils::asset_plugin_at;
use gdtf_battle_presenter::{CrossLevelBadgeLabel, CrossLevelBadgeTile, TopDownRendererPlugin};
use gdtf_battle_sim::prelude::BattleInProgress;

pub(crate) const MAX_UPDATES: u32 = 16;

fn workspace_assets_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

pub(crate) fn signals_app() -> App {
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
            .set(asset_plugin_at(&workspace_assets_root())),
    )
    .add_plugins(TopDownRendererPlugin);
    app.insert_resource(BattleInProgress);
    app.set_error_handler(warn);
    app
}

pub(crate) fn settle(app: &mut App) {
    for _ in 0..MAX_UPDATES {
        app.update();
    }
}

pub(crate) fn visible_tile_count(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<&Visibility, With<CrossLevelBadgeTile>>()
        .iter(app.world())
        .filter(|v| **v == Visibility::Visible)
        .count()
}

pub(crate) fn visible_label_texts(app: &mut App) -> Vec<String> {
    let mut q = app
        .world_mut()
        .query::<(&Text2d, &Visibility, &CrossLevelBadgeLabel)>();
    q.iter(app.world())
        .filter(|(_, vis, _)| **vis == Visibility::Visible)
        .map(|(text, ..)| (**text).clone())
        .collect()
}
