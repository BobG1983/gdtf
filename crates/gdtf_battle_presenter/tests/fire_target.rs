//! GTW-371 (C2 / C4): headless draw-LOGIC proof for the fire-target highlight — the POSITIVE
//! in-engine state assertions the contract demands.
//!
//! C2 / C4b (RED TILE + OPAQUE COST, positive): with a `FireTargetHighlight` on a NAMED cell +
//! cost, the presenter draws a `Visibility::Visible` `FireTargetTile` `Sprite` at the NAMED cell
//! whose draw-z is UNDER the actor band (the contract's "rendered UNDER the enemy sprite"), PLUS
//! a single `Visibility::Visible` `FireTargetLabel` `Text2d` over the NAMED cell reading the fire
//! cost as `"N TU"` in an OPAQUE colour.
//! C4c (CLEARS): clearing the highlight HIDES the tile AND the cost label (mutate-not-respawn).
//! C5 (HARD CUT): a fire target on a DIFFERENT storey is NOT drawn on the active storey.
//!
//! This is the `path_preview.rs` pattern: a `DefaultPlugins`/`no_renderer` app with the real
//! `TopDownRendererPlugin`. The fire-target draw system gates on `BattleInProgress` (a solid-tint
//! sprite + a `Text2d`, no atlas, no `SquadVisibility`), so the headless app drives the REAL draw
//! path. The `FireTargetHighlight` resource is authored DIRECTLY via `app.world_mut()` in the test
//! body (the accepted headless idiom, `bevy-traps.md` #7 carve-out (a)) — standing in for the
//! input crate's populate system, which is unit-tested in `gdtf_battle_input` over the same
//! resource.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::error::warn,
    prelude::{Alpha, Text2d, Transform, Visibility, default},
    render::{RenderPlugin, settings::WgpuSettings},
    text::TextColor,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    FireTargetHighlight, FireTargetLabel, FireTargetTile, Layer, TopDownRendererPlugin,
    cell_to_world, cell_to_world_layered,
};
use gdtf_battle_sim::prelude::{BattleInProgress, Cell, CellLevel, Level, Tu};

/// Bounded settle headroom for the deferred draw (a synchronous command flush, not a load).
const MAX_UPDATES: u32 = 16;

/// The workspace-root `assets/` directory (this crate's manifest -> up two -> assets).
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// The headless `DefaultPlugins`/`no_renderer` app with the real `TopDownRendererPlugin` plus a
/// `BattleInProgress` witness so the battle-gated fire-target draw system runs.
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
            .set(AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .add_plugins(TopDownRendererPlugin);
    app.insert_resource(BattleInProgress);
    // Warn-not-panic on a transient missing-resource gate race (the `path_preview.rs`
    // precedent): the focused harness opens `BattleInProgress` WITHOUT the full `setup_battle`,
    // so other battle-gated draw systems warn-skip rather than panicking. The fire-target draw
    // reads the `init_resource`-d `FireTargetHighlight` + `ActiveLevel`, so it always runs here.
    app.set_error_handler(warn);
    app
}

/// Author the fire-target highlight directly (standing in for the input populate system).
fn set_highlight(app: &mut App, cell: CellLevel, cost: Tu) {
    app.world_mut()
        .insert_resource(FireTargetHighlight::new(cell, cost));
}

/// Drive bounded `update()`s until the SINGLE `FireTargetTile` has materialized (the lazy pool
/// spawn lands in the end-of-update command flush).
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

/// Planar-position equality within float noise (world positions are exact `CELL_PX` multiples).
fn planar_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// The `(planar-matches-cell, draw-z, visible)` of the SINGLE fire-target tile, if it exists.
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

/// Count of `Visible` `FireTargetTile`s — must never exceed ONE (the single fire-target tile).
fn visible_tile_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &FireTargetTile)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

/// The `(text, planar-matches-cell, above-cell, opaque, visible)` of the SINGLE cost label, if it
/// exists — the cost-on-target witness (the label is lifted ABOVE the cell, OPAQUE).
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

/// Count of `Visible` `FireTargetLabel`s — must never exceed ONE.
fn visible_label_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &FireTargetLabel)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

/// C2 / C4b (positive) — with a fire-target highlight on a NAMED cell, the presenter draws a
/// `Visible` red tile UNDER the actor band at the NAMED cell, plus a single OPAQUE cost label
/// over it reading the fire cost as `"N TU"`.
#[test]
fn fire_target_tile_under_actor_and_opaque_cost_label() {
    let mut app = fire_target_app();
    let l0 = Level::new(0);
    // A NAMED fireable-enemy cell + a NAMED cost (the cost a fire would charge).
    let cell = CellLevel::new(Cell::new(13, 9), l0);
    let cost = Tu::new(14);

    set_highlight(&mut app, cell, cost);
    assert!(
        settle_tile(&mut app),
        "the fire-target tile must have drawn"
    );

    // POSITIVE: exactly ONE visible tile, over the NAMED cell, drawn UNDER the actor band.
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
    // UNDER the actor: the tile's draw-z is strictly below the Actor band's z at the same cell.
    let actor_z = cell_to_world_layered(cell.cell(), l0, Layer::Actor).z;
    assert!(
        z < actor_z,
        "the fire-target tile draws UNDER the actor band (z {z} < actor z {actor_z}) so it \
         renders BELOW the enemy sprite",
    );
    // Sanity: it is exactly the FireTarget band z (the under-actor band, 0.075 over level z).
    let want_z = cell_to_world_layered(cell.cell(), l0, Layer::FireTarget).z;
    assert!(
        (z - want_z).abs() < 0.001,
        "the tile is at the FireTarget band (z {z} == FireTarget band z {want_z})",
    );

    // POSITIVE: exactly ONE visible OPAQUE cost label, over the NAMED cell, reading the cost.
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

/// C4c (CLEARS) — clearing the highlight HIDES the tile AND the cost label (mutate-not-respawn:
/// the pooled entities persist but go `Hidden`), so moving off a fireable enemy leaves no stale
/// fire target.
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

    // Clear (not hovering a fireable enemy) — the pooled tile + label must hide, not linger.
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

/// C5 (HARD CUT) — a fire target on a DIFFERENT storey is NOT drawn on the active storey (the
/// tile + label hard-cut to the active storey).
#[test]
fn fire_target_hard_cut_when_off_storey() {
    let mut app = fire_target_app();
    let l1 = Level::new(1);
    // The active storey defaults to level 0; the fire target is on level 1.
    let off_storey = CellLevel::new(Cell::new(8, 5), l1);

    set_highlight(&mut app, off_storey, Tu::new(12));
    // Drive a few updates so the draw system runs (the pool may never spawn if always hidden).
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
