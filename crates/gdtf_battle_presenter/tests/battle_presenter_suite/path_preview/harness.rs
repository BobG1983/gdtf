//! Shared `path_preview` fixture: the preview app, preview / vision authoring, and
use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    ecs::error::warn,
    platform::collections::HashSet,
    prelude::{Text2d, Transform, Visibility, default},
    render::{RenderPlugin, settings::WgpuSettings},
    time::TimeUpdateStrategy,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use cobalt_test_utils::{advance_until_mut, asset_plugin_at};
use gdtf_battle_presenter::{
    PathPreview, PathStepSprite, PathTargetLabel, TopDownRendererPlugin, cell_to_world,
};
use gdtf_battle_sim::{
    prelude::{BattleInProgress, CellLevel, Tu},
    visibility::SquadVisibility,
};

use crate::pinned_delta::{PINNED_DELTA, assert_reads_a_pinned_delta};

pub(crate) fn workspace_assets_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

pub(crate) fn full_vision(cells: &[CellLevel]) -> SquadVisibility {
    let all: HashSet<CellLevel> = cells.iter().copied().collect();
    SquadVisibility::new(all.clone(), all)
}

pub(crate) fn preview_app() -> App {
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
    app.insert_resource(TimeUpdateStrategy::ManualDuration(PINNED_DELTA));
    app.set_error_handler(warn);
    app
}

#[test]
fn the_harness_app_reads_a_pinned_delta() {
    assert_reads_a_pinned_delta(&mut preview_app());
}

pub(crate) fn set_preview(app: &mut App, cells: Vec<CellLevel>, cost: Tu) {
    app.world_mut().insert_resource(full_vision(&cells));
    app.world_mut()
        .insert_resource(PathPreview::new(cells, cost));
}

pub(crate) fn settle_steps(app: &mut App) {
    advance_until_mut(app, |app| {
        let mut q = app.world_mut().query::<&PathStepSprite>();
        q.iter(app.world()).next().is_some()
    });
}

pub(crate) fn step_visible_at(app: &mut App, cell: CellLevel) -> bool {
    let (want_cell, want_level) = cell.split();
    let want = cell_to_world(want_cell, want_level);
    let mut q = app
        .world_mut()
        .query::<(&Transform, &Visibility, &PathStepSprite)>();
    q.iter(app.world()).any(|(t, vis, _)| {
        planar_eq(t.translation.x, want.x)
            && planar_eq(t.translation.y, want.y)
            && *vis == Visibility::Visible
    })
}

pub(crate) fn visible_step_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &PathStepSprite)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

pub(crate) fn settle_label(app: &mut App) {
    advance_until_mut(app, |app| {
        let mut q = app.world_mut().query::<&PathTargetLabel>();
        q.iter(app.world()).next().is_some()
    });
}

pub(crate) fn target_label_state(app: &mut App, target: CellLevel) -> Option<(String, bool, bool)> {
    let (want_cell, want_level) = target.split();
    let want = cell_to_world(want_cell, want_level);
    let mut q = app
        .world_mut()
        .query::<(&Text2d, &Transform, &Visibility, &PathTargetLabel)>();
    q.iter(app.world()).next().map(|(text, t, vis, _)| {
        let over_target = planar_eq(t.translation.x, want.x) && t.translation.y > want.y;
        ((**text).clone(), over_target, *vis == Visibility::Visible)
    })
}

pub(crate) fn visible_label_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &PathTargetLabel)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

pub(crate) fn planar_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}
