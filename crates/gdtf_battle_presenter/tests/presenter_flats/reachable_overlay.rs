//! Reachable overlay draw: sprites match the reachable set and hide when cleared.
use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::error::warn,
    platform::collections::HashSet,
    prelude::{Transform, Visibility, default},
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    ActiveLevel, Layer, ReachableCellSprite, ReachableCells, ReachableOverlayEnabled,
    TopDownRendererPlugin, cell_to_world_layered,
};
use gdtf_battle_sim::{
    prelude::{BattleInProgress, Cell, CellLevel, Level, Tu},
    visibility::SquadVisibility,
};

const MAX_UPDATES: u32 = 16;

fn workspace_assets_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

fn full_vision(cells: &[CellLevel]) -> SquadVisibility {
    let all: HashSet<CellLevel> = cells.iter().copied().collect();
    SquadVisibility::new(all.clone(), all)
}

fn overlay_app() -> App {
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
    )
    .add_plugins(TopDownRendererPlugin);
    app.insert_resource(BattleInProgress);
    app.insert_resource(ReachableOverlayEnabled::new(true));
    app.set_error_handler(warn);
    app
}

fn set_reachable(app: &mut App, cells: Vec<(CellLevel, Tu)>, active: Level) {
    let level_cells: Vec<CellLevel> = cells.iter().map(|(cell, _)| *cell).collect();
    app.world_mut().insert_resource(full_vision(&level_cells));
    app.world_mut().insert_resource(ReachableCells::new(cells));
    app.world_mut().insert_resource(ActiveLevel::new(active));
}

fn settle_sprites(app: &mut App) -> bool {
    for _ in 0..MAX_UPDATES {
        let mut q = app.world_mut().query::<&ReachableCellSprite>();
        if q.iter(app.world()).next().is_some() {
            return true;
        }
        app.update();
    }
    let mut q = app.world_mut().query::<&ReachableCellSprite>();
    q.iter(app.world()).next().is_some()
}

fn sprite_visible_at(app: &mut App, cell: CellLevel) -> bool {
    let (want_cell, want_level) = cell.split();
    let want = cell_to_world_layered(want_cell, want_level, Layer::ReachableRange);
    let mut q = app
        .world_mut()
        .query::<(&Transform, &Visibility, &ReachableCellSprite)>();
    q.iter(app.world()).any(|(t, vis, _)| {
        approx_eq(t.translation.x, want.x)
            && approx_eq(t.translation.y, want.y)
            && approx_eq(t.translation.z, want.z)
            && *vis == Visibility::Visible
    })
}

fn visible_sprite_count(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query::<(&Visibility, &ReachableCellSprite)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn reachable_overlay_renders_on_upper_storey_after_level_switch() {
    let mut app = overlay_app();

    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let on0_a = CellLevel::new(Cell::new(5, 5), l0);
    let on0_b = CellLevel::new(Cell::new(6, 5), l0);
    let on1_a = CellLevel::new(Cell::new(20, 20), l1);
    let on1_b = CellLevel::new(Cell::new(21, 20), l1);

    set_reachable(
        &mut app,
        vec![
            (on0_a, Tu::new(4)),
            (on0_b, Tu::new(8)),
            (on1_a, Tu::new(20)),
            (on1_b, Tu::new(24)),
        ],
        l1,
    );
    assert!(
        settle_sprites(&mut app),
        "the reachable-range sprites must have drawn on the active L1 storey",
    );

    assert!(
        sprite_visible_at(&mut app, on1_a),
        "L1 reachable cell (20,20,L1) must render on the upper storey after the switch",
    );
    assert!(
        sprite_visible_at(&mut app, on1_b),
        "L1 reachable cell (21,20,L1) must render on the upper storey after the switch",
    );
    assert_eq!(
        visible_sprite_count(&mut app),
        2,
        "exactly the two L1 reachable cells are drawn on L1 (the L0 cells are hard-cut)",
    );
    assert!(
        !sprite_visible_at(&mut app, CellLevel::new(Cell::new(5, 5), l1)),
        "an L0 reachable cell must NOT render on the active L1 storey (the hard cut)",
    );
    assert!(
        !sprite_visible_at(&mut app, CellLevel::new(Cell::new(6, 5), l1)),
        "the second L0 reachable cell is hard-cut on L1 too",
    );
}
