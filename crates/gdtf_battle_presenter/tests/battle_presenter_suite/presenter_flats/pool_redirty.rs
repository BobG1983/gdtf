//! Path preview pool: surplus hidden sprites must not redirty Visibility every frame.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup, Update},
    ecs::error::warn,
    platform::collections::HashSet,
    prelude::{
        Deref, DerefMut, DetectChanges, IntoScheduleConfigs, Query, Ref, ResMut, Resource,
        Visibility, With, default,
    },
    render::{RenderPlugin, settings::WgpuSettings},
    time::TimeUpdateStrategy,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use cobalt_test_utils::{advance_until_mut, asset_plugin_at};
use gdtf_battle_presenter::{PathPreview, PathStepSprite, PresenterSystems, TopDownRendererPlugin};
use gdtf_battle_sim::{
    prelude::{BattleInProgress, Cell, CellLevel, Level, Tu},
    visibility::SquadVisibility,
};

use crate::pinned_delta::{PINNED_DELTA, assert_reads_a_pinned_delta};

fn workspace_assets_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

#[derive(Resource, Default, Deref, DerefMut)]
struct HiddenRedirtyCount(usize);

fn record_hidden_redirty(
    pooled: Query<Ref<Visibility>, With<PathStepSprite>>,
    mut count: ResMut<HiddenRedirtyCount>,
) {
    **count = pooled
        .iter()
        .filter(|vis| **vis == Visibility::Hidden && vis.is_changed())
        .count();
}

fn full_vision(cells: &[CellLevel]) -> SquadVisibility {
    let all: HashSet<CellLevel> = cells.iter().copied().collect();
    SquadVisibility::new(all.clone(), all)
}

fn probe_app() -> App {
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
    app.init_resource::<HiddenRedirtyCount>();
    app.add_systems(Update, record_hidden_redirty.after(PresenterSystems::Draw));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(PINNED_DELTA));
    app.set_error_handler(warn);
    app
}

#[test]
fn the_harness_app_reads_a_pinned_delta() {
    assert_reads_a_pinned_delta(&mut probe_app());
}

fn set_preview(app: &mut App, cells: Vec<CellLevel>, cost: Tu) {
    app.world_mut().insert_resource(full_vision(&cells));
    app.world_mut()
        .insert_resource(PathPreview::new(cells, cost));
}

fn settle_pool_size(app: &mut App, want: usize) {
    advance_until_mut(app, |app| {
        let mut q = app.world_mut().query::<&PathStepSprite>();
        q.iter(app.world()).count() >= want
    });
}

fn hidden_step_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &PathStepSprite)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Hidden)
        .count()
}

#[test]
fn steady_frame_leaves_surplus_hidden_visibility_ticks_untouched() {
    let mut app = probe_app();
    let l0 = Level::new(0);
    let a = CellLevel::new(Cell::new(5, 5), l0);
    let b = CellLevel::new(Cell::new(6, 5), l0);
    let c = CellLevel::new(Cell::new(7, 5), l0);

    set_preview(&mut app, vec![a, b, c], Tu::new(12));
    settle_pool_size(&mut app, 3);
    set_preview(&mut app, vec![a], Tu::new(4));
    app.update();
    assert_eq!(
        hidden_step_count(&mut app),
        2,
        "the shrunk route leaves two surplus pooled sprites hidden",
    );

    app.update();
    let redirtied = app.world().resource::<HiddenRedirtyCount>();
    assert_eq!(
        **redirtied, 0,
        "a steady frame with unchanged draws must leave every surplus hidden pooled \
         sprite's Visibility change ticks untouched (the pre-refactor unconditional \
         `*visibility = Hidden` write re-dirtied them every frame)",
    );
}
