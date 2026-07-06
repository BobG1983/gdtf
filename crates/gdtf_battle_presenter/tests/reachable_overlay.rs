//! GTW-387 C3 (DRAW side, positive): headless presenter integration proof that the
//! reachable-range overlay actually RENDERS on the upper storey after a level switch.
//!
//! The sibling pure-logic tests (`overlays/reachable/test.rs`) pin the read-seam +
//! the `reachable_draws` active-storey hard-cut WITHOUT an app, and the input crate
//! unit-tests the producer (`ReachableCells` population). What no test covered until
//! now is the DRAW system spawning a `Visible`, world-positioned `ReachableCellSprite`
//! on the active storey — the in-engine evidence the C3 acceptance demands.
//!
//! This is the `path_preview.rs` pattern: a `DefaultPlugins`/`no_renderer` app with the
//! real `TopDownRendererPlugin`. The `draw_reachable_overlay` system gates on
//! `BattleInProgress` + `SquadVisibility` (a solid-tint sprite, no atlas), so the
//! headless app drives the REAL draw path. The `ReachableCells` + `SquadVisibility` +
//! `ActiveLevel` resources are authored DIRECTLY via `app.world_mut()` in the test body
//! (the accepted headless idiom, `bevy-traps.md` #7 carve-out (a)) — standing in for the
//! input crate's populate system, which is unit-tested in `gdtf_battle_input` over the
//! same resource.

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

/// Bounded settle headroom for the deferred draw (a synchronous command flush, not a load).
const MAX_UPDATES: u32 = 16;

/// The workspace-root `assets/` directory (this crate's manifest -> up two -> assets).
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// A `SquadVisibility` with every passed cell VISIBLE + EXPLORED — so squad fog is not the
/// variable under test (the gate the draw system needs is the resource's PRESENCE; the
/// reachable overlay does not itself fog per cell, but the system's run-condition requires
/// `SquadVisibility` to exist).
fn full_vision(cells: &[CellLevel]) -> SquadVisibility {
    let all: HashSet<CellLevel> = cells.iter().copied().collect();
    SquadVisibility::new(all.clone(), all)
}

/// The headless `DefaultPlugins`/`no_renderer` app with the real `TopDownRendererPlugin`
/// (the `path_preview.rs` harness) plus a `BattleInProgress` witness so the battle-gated
/// reachable-overlay draw system runs.
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
    // GTW-450 — the draw system `run_if`s the `ReachableOverlayEnabled` flag VALUE; the
    // plugin seeds it from the env var (default OFF). Force it ON via the RESOURCE directly
    // (NEVER process-global env, the flaky-tests rule) so this in-engine draw proof runs.
    app.insert_resource(ReachableOverlayEnabled::new(true));
    // Warn-not-panic on a transient missing-resource gate race (the path_preview.rs
    // precedent): the focused harness opens `BattleInProgress` WITHOUT the full
    // `setup_battle`, so other battle-gated draw systems warn-skip rather than panicking.
    // The reachable draw reads the `init_resource`-d `ReachableCells` + `ActiveLevel` and
    // the `SquadVisibility` the test seeds, so it always runs here.
    app.set_error_handler(warn);
    app
}

/// Author the reachable set + the squad fog + the active storey directly (standing in for
/// the input populate system + the `PageUp` level switch).
fn set_reachable(app: &mut App, cells: Vec<(CellLevel, Tu)>, active: Level) {
    let level_cells: Vec<CellLevel> = cells.iter().map(|(cell, _)| *cell).collect();
    app.world_mut().insert_resource(full_vision(&level_cells));
    app.world_mut().insert_resource(ReachableCells::new(cells));
    app.world_mut().insert_resource(ActiveLevel::new(active));
}

/// Drive bounded `update()`s until at least one `ReachableCellSprite` has materialized (the
/// lazy pool spawn lands in the end-of-update command flush).
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

/// Whether a `ReachableCellSprite` is `Visible` at the world position of `cell` on its own
/// storey — the presenter's drawn-cell witness (matches the full layered world position the
/// draw system computes via `cell_to_world_layered` at the `ReachableRange` band).
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

/// Count of `Visible` `ReachableCellSprite`s (so the active-storey hard-cut can be pinned).
fn visible_sprite_count(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query::<(&Visibility, &ReachableCellSprite)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

/// World-position equality within float noise (world positions are exact multiples).
fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// C3 (DRAW, positive, upper storey) — with the active level switched to L1 and the
/// cross-storey reachable set holding both L0 and L1 cells, the presenter draws a
/// `Visible`, correctly-positioned `ReachableCellSprite` at each L1 reachable cell, and
/// the L0 cells are hard-cut (NOT drawn on the upper storey). This proves the overlay
/// RENDERS on the upper storey after the level switch — not merely that the resource was
/// populated.
#[test]
fn reachable_overlay_renders_on_upper_storey_after_level_switch() {
    let mut app = overlay_app();

    let l0 = Level::new(0);
    let l1 = Level::new(1);
    // L0 cells (the ganger's start storey) + L1 cells (the upper storey the player switches
    // to). The L1 cells sit at planar columns DISTINCT from the L0 cells so the hard-cut is
    // unambiguous.
    let on0_a = CellLevel::new(Cell::new(5, 5), l0);
    let on0_b = CellLevel::new(Cell::new(6, 5), l0);
    let on1_a = CellLevel::new(Cell::new(20, 20), l1); // the stair head (arrival)
    let on1_b = CellLevel::new(Cell::new(21, 20), l1); // an L1 platform cell

    // Switch the active storey to L1 (the `PageUp` the acceptance requires) with the full
    // cross-storey reachable set.
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

    // POSITIVE: each L1 reachable cell is drawn `Visible` at its own layered world position.
    assert!(
        sprite_visible_at(&mut app, on1_a),
        "L1 reachable cell (20,20,L1) must render on the upper storey after the switch",
    );
    assert!(
        sprite_visible_at(&mut app, on1_b),
        "L1 reachable cell (21,20,L1) must render on the upper storey after the switch",
    );
    // Exactly the two L1 cells are drawn (the active-storey hard-cut: the two L0 cells are
    // excluded when active = L1).
    assert_eq!(
        visible_sprite_count(&mut app),
        2,
        "exactly the two L1 reachable cells are drawn on L1 (the L0 cells are hard-cut)",
    );
    // HARD CUT control: the L0 cells are NOT drawn on the active L1 storey (at their own
    // planar columns, which no L1 sprite occupies).
    assert!(
        !sprite_visible_at(&mut app, CellLevel::new(Cell::new(5, 5), l1)),
        "an L0 reachable cell must NOT render on the active L1 storey (the hard cut)",
    );
    assert!(
        !sprite_visible_at(&mut app, CellLevel::new(Cell::new(6, 5), l1)),
        "the second L0 reachable cell is hard-cut on L1 too",
    );
}
