//! Fire target tile: highlight visibility and label state for the aim cell.
use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    ecs::error::warn,
    prelude::{Alpha, Text2d, Transform, Visibility, default},
    render::{RenderPlugin, settings::WgpuSettings},
    text::TextColor,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use cobalt_test_utils::asset_plugin_at;
use gdtf_battle_presenter::{
    FireTargetHighlight, FireTargetLabel, FireTargetTile, Layer, TopDownRendererPlugin,
    cell_to_world, cell_to_world_layered,
};
use gdtf_battle_sim::prelude::{BattleInProgress, Cell, CellLevel, Level, Tu};

const MAX_UPDATES: u32 = 16;

fn workspace_assets_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

fn fire_target_app() -> App {
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

fn set_highlight(app: &mut App, cell: CellLevel, cost: Tu) {
    app.world_mut()
        .insert_resource(FireTargetHighlight::new(cell, cost));
}

fn settle_tile(app: &mut App) -> bool {
    for _ in 0..MAX_UPDATES {
        let mut q = app.world_mut().query::<&FireTargetTile>();
        if q.iter(app.world()).next().is_some() {
            return true;
        }
        app.update();
    }
    let mut q = app.world_mut().query::<&FireTargetTile>();
    q.iter(app.world()).next().is_some()
}

fn planar_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn tile_state(app: &mut App, cell: CellLevel) -> Option<(bool, f32, bool)> {
    let (want_cell, want_level) = cell.split();
    let want = cell_to_world(want_cell, want_level);
    let mut q = app
        .world_mut()
        .query::<(&Transform, &Visibility, &FireTargetTile)>();
    q.iter(app.world()).next().map(|(t, vis, _)| {
        let over_cell = planar_eq(t.translation.x, want.x) && planar_eq(t.translation.y, want.y);
        (over_cell, t.translation.z, *vis == Visibility::Visible)
    })
}

fn visible_tile_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &FireTargetTile)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

fn label_state(app: &mut App, cell: CellLevel) -> Option<(String, bool, bool, bool, bool)> {
    let (want_cell, want_level) = cell.split();
    let want = cell_to_world(want_cell, want_level);
    let mut q = app.world_mut().query::<(
        &Text2d,
        &Transform,
        &TextColor,
        &Visibility,
        &FireTargetLabel,
    )>();
    q.iter(app.world()).next().map(|(text, t, color, vis, _)| {
        let over_cell = planar_eq(t.translation.x, want.x);
        let above = t.translation.y > want.y;
        let opaque = (color.0.alpha() - 1.0).abs() < 0.001;
        (
            (**text).clone(),
            over_cell,
            above,
            opaque,
            *vis == Visibility::Visible,
        )
    })
}

fn visible_label_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &FireTargetLabel)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

#[test]
fn fire_target_tile_under_actor_and_opaque_cost_label() {
    let mut app = fire_target_app();
    let l0 = Level::new(0);
    let cell = CellLevel::new(Cell::new(13, 9), l0);
    let cost = Tu::new(14);

    set_highlight(&mut app, cell, cost);
    assert!(
        settle_tile(&mut app),
        "the fire-target tile must have drawn"
    );

    assert_eq!(
        visible_tile_count(&mut app),
        1,
        "exactly ONE fire-target tile is drawn (on the hovered enemy cell only)",
    );
    let (over_cell, z, visible) = tile_state(&mut app, cell).unwrap_or((false, f32::NAN, false));
    assert!(
        visible,
        "the fire-target tile is Visible while hovering a fireable enemy"
    );
    assert!(
        over_cell,
        "the red tile is positioned at the NAMED enemy cell (13,9,L0)"
    );
    let actor_z = cell_to_world_layered(cell.cell(), l0, Layer::Actor).z;
    assert!(
        z < actor_z,
        "the fire-target tile draws UNDER the actor band (z {z} < actor z {actor_z}) so it \
         renders BELOW the enemy sprite",
    );
    let want_z = cell_to_world_layered(cell.cell(), l0, Layer::FireTarget).z;
    assert!(
        (z - want_z).abs() < 0.001,
        "the tile is at the FireTarget band (z {z} == FireTarget band z {want_z})",
    );

    assert_eq!(
        visible_label_count(&mut app),
        1,
        "exactly ONE cost label is drawn (on the hovered enemy cell only)",
    );
    let (text, over_cell, above, opaque, visible) = label_state(&mut app, cell).unwrap_or_default();
    assert!(
        visible,
        "the cost label is Visible while hovering a fireable enemy"
    );
    assert!(
        over_cell && above,
        "the cost label is positioned ABOVE the NAMED enemy cell"
    );
    assert!(
        opaque,
        "the cost label is OPAQUE (alpha 1.0), like the move-cost label (C2)"
    );
    assert_eq!(
        text, "14 TU",
        "the cost label reads the fire cost (mode_tu_cost) as \"14 TU\"",
    );
}

#[test]
fn clearing_highlight_hides_tile_and_cost_label() {
    let mut app = fire_target_app();
    let l0 = Level::new(0);
    let cell = CellLevel::new(Cell::new(7, 7), l0);

    set_highlight(&mut app, cell, Tu::new(10));
    assert!(settle_tile(&mut app), "the fire target must have drawn");
    assert_eq!(
        visible_tile_count(&mut app),
        1,
        "the tile is shown before clear"
    );
    assert_eq!(
        visible_label_count(&mut app),
        1,
        "the label is shown before clear"
    );

    app.world_mut()
        .insert_resource(FireTargetHighlight::cleared());
    app.update();

    assert_eq!(
        visible_tile_count(&mut app),
        0,
        "after the highlight clears, no fire-target tile stays drawn (mutate-not-respawn hide)",
    );
    assert_eq!(
        visible_label_count(&mut app),
        0,
        "after the highlight clears, the cost label hides (mutate-not-respawn)",
    );
}

#[test]
fn fire_target_hard_cut_when_off_storey() {
    let mut app = fire_target_app();
    let l1 = Level::new(1);
    let off_storey = CellLevel::new(Cell::new(8, 5), l1);

    set_highlight(&mut app, off_storey, Tu::new(12));
    for _ in 0..MAX_UPDATES {
        app.update();
    }

    assert_eq!(
        visible_tile_count(&mut app),
        0,
        "no fire-target tile is drawn when the target cell is off the active storey (hard cut)",
    );
    assert_eq!(
        visible_label_count(&mut app),
        0,
        "no cost label is drawn when the target cell is off the active storey (hard cut)",
    );
}
