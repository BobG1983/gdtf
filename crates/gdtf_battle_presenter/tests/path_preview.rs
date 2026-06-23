//! GTW-358 / GTW-368 (C1 / C2 / C3 / C4 / C5 / C7): headless draw-LOGIC proof for the route
//! path-preview — the POSITIVE in-engine state assertions the contract demands.
//!
//! C1 (ROUTE, positive): with a `PathPreview` route at NAMED cells, the presenter draws a
//! `Visibility::Visible`, world-positioned `PathStepSprite` at each NAMED route cell ON the
//! active storey. The assertion NAMES the route cells that must appear.
//! C2 (GTW-368 COST-ON-TARGET, positive): with a previewed route, the presenter draws a SINGLE
//! `Visibility::Visible` `PathTargetLabel` `Text2d` at the NAMED target cell (the route's last
//! cell) reading the route cost as `"N TU"`; with NO target the label is HIDDEN.
//! C5 (HARD CUT): an off-`ActiveLevel` route cell is NOT drawn (and the route leaving the
//! storey draws a minimal marker at the link cell).
//! C4 (CLEARS / idle board): clearing the preview HIDES every step AND the cost label
//! (mutate-not-respawn), and the IDLE board has NO visible overlay entity (no per-cell labels —
//! the GTW-357 reachable overlay was removed entirely).
//!
//! This is the `fog_present.rs` pattern: a `DefaultPlugins`/`no_renderer` app with the real
//! `TopDownRendererPlugin`. The path-preview draw system gates on `BattleInProgress` +
//! `SquadVisibility` (a solid-tint sprite + a `Text2d`, no atlas), so the headless app drives
//! the REAL draw path. The `PathPreview` + `SquadVisibility` resources are authored DIRECTLY via
//! `app.world_mut()` in the test body (the accepted headless idiom, `bevy-traps.md` #7 carve-out
//! (a)) — standing in for the input crate's populate system, which is unit-tested in
//! `gdtf_battle_input` over the same resource.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::error::warn,
    platform::collections::HashSet,
    prelude::{Text2d, Transform, Visibility, default},
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    PathPreview, PathStepSprite, PathTargetLabel, TopDownRendererPlugin, cell_to_world,
};
use gdtf_battle_sim::{BattleInProgress, Cell, CellLevel, Level, SquadVisibility, Tu};

/// Bounded settle headroom for the deferred draw (a synchronous command flush, not a load).
const MAX_UPDATES: u32 = 16;

/// The workspace-root `assets/` directory (this crate's manifest -> up two -> assets).
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// A `SquadVisibility` with every passed cell VISIBLE + EXPLORED — so §53 is not the variable
/// under test (the §53 VISIBLE-vs-EXPLORED dim is the pure-logic `preview_draws` unit test).
fn full_vision(cells: &[CellLevel]) -> SquadVisibility {
    let all: HashSet<CellLevel> = cells.iter().copied().collect();
    SquadVisibility::new(all.clone(), all)
}

/// The headless `DefaultPlugins`/`no_renderer` app with the real `TopDownRendererPlugin` (the
/// `reachable_overlay.rs` harness) plus a `BattleInProgress` witness so the battle-gated
/// path-preview draw system runs.
fn preview_app() -> App {
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
    // Warn-not-panic on a transient missing-resource gate race (the `reachable_overlay.rs`
    // precedent): the focused harness opens `BattleInProgress` WITHOUT the full `setup_battle`,
    // so other battle-gated draw systems warn-skip rather than panicking. The path-preview draw
    // reads the `init_resource`-d `PathPreview` + `ActiveLevel` + the `SquadVisibility` the test
    // seeds, so it always runs here.
    app.set_error_handler(warn);
    app
}

/// Author the path preview + the squad fog directly (standing in for the input populate system).
fn set_preview(app: &mut App, cells: Vec<CellLevel>, cost: Tu) {
    app.world_mut().insert_resource(full_vision(&cells));
    app.world_mut()
        .insert_resource(PathPreview::new(cells, cost));
}

/// Drive bounded `update()`s until at least one `PathStepSprite` has materialized (the lazy
/// pool spawn lands in the end-of-update command flush).
fn settle_steps(app: &mut App) -> bool {
    for _ in 0..MAX_UPDATES {
        let mut q = app.world_mut().query::<&PathStepSprite>();
        if q.iter(app.world()).next().is_some() {
            return true;
        }
        app.update();
    }
    let mut q = app.world_mut().query::<&PathStepSprite>();
    q.iter(app.world()).next().is_some()
}

/// Whether a `PathStepSprite` is `Visible` at the planar world position of `cell` — the
/// presenter's drawn-step witness (matches on the planar `(x, y)`; the layer-z is not part of
/// cell identity).
fn step_visible_at(app: &mut App, cell: CellLevel) -> bool {
    let want = cell_to_world(Cell::new(cell.x, cell.y), Level::new(level_u8(cell)));
    let mut q = app
        .world_mut()
        .query::<(&Transform, &Visibility, &PathStepSprite)>();
    q.iter(app.world()).any(|(t, vis, _)| {
        planar_eq(t.translation.x, want.x)
            && planar_eq(t.translation.y, want.y)
            && *vis == Visibility::Visible
    })
}

/// Count of `Visible` `PathStepSprite`s (so the hard cut + the surplus-hide can be pinned).
fn visible_step_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &PathStepSprite)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

/// Drive bounded `update()`s until the SINGLE `PathTargetLabel` has materialized (GTW-368).
fn settle_label(app: &mut App) -> bool {
    for _ in 0..MAX_UPDATES {
        let mut q = app.world_mut().query::<&PathTargetLabel>();
        if q.iter(app.world()).next().is_some() {
            return true;
        }
        app.update();
    }
    let mut q = app.world_mut().query::<&PathTargetLabel>();
    q.iter(app.world()).next().is_some()
}

/// The `(text, planar-position-matches-target, visible)` of the SINGLE target-cost label, if it
/// exists — the GTW-368 cost-on-target witness (the label is lifted ABOVE the cell, so it
/// matches the target's planar `x` and a `y` above the cell centre).
fn target_label_state(app: &mut App, target: CellLevel) -> Option<(String, bool, bool)> {
    let want = cell_to_world(Cell::new(target.x, target.y), Level::new(level_u8(target)));
    let mut q = app
        .world_mut()
        .query::<(&Text2d, &Transform, &Visibility, &PathTargetLabel)>();
    q.iter(app.world()).next().map(|(text, t, vis, _)| {
        let over_target = planar_eq(t.translation.x, want.x) && t.translation.y > want.y;
        ((**text).clone(), over_target, *vis == Visibility::Visible)
    })
}

/// Count of `Visible` `PathTargetLabel`s — must never exceed ONE (the single target cost label).
fn visible_label_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&Visibility, &PathTargetLabel)>();
    q.iter(app.world())
        .filter(|(vis, _)| **vis == Visibility::Visible)
        .count()
}

/// The storey index narrowed to the `u8` a `Level` carries (the cells under test are within `u8`).
fn level_u8(cell: CellLevel) -> u8 {
    u8::try_from(cell.z).unwrap_or(u8::MAX)
}

/// Planar-position equality within float noise (world positions are exact `CELL_PX` multiples).
fn planar_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// C1 (positive) — the NAMED route cells on the active storey are drawn (a `Visible`,
/// correctly-positioned `PathStepSprite`), and a cell NOT in the route has no lit step.
#[test]
fn route_cells_drawn_nonroute_cell_dark() {
    let mut app = preview_app();

    let l0 = Level::new(0);
    // A NAMED route across the active storey (level 0) + a NAMED cell NOT in the route.
    let a = CellLevel::new(Cell::new(10, 10), l0);
    let b = CellLevel::new(Cell::new(11, 10), l0);
    let c = CellLevel::new(Cell::new(12, 10), l0);
    let off_route = CellLevel::new(Cell::new(40, 40), l0);

    set_preview(&mut app, vec![a, b, c], Tu::new(12));
    assert!(
        settle_steps(&mut app),
        "the route step sprites must have drawn"
    );

    // POSITIVE: each NAMED route cell is drawn at its own world position.
    assert!(
        step_visible_at(&mut app, a),
        "route cell (10,10,L0) must be drawn"
    );
    assert!(
        step_visible_at(&mut app, b),
        "route cell (11,10,L0) must be drawn"
    );
    assert!(
        step_visible_at(&mut app, c),
        "route cell (12,10,L0) must be drawn"
    );
    // Exactly the three flat route cells are drawn (no extra link marker on a flat route).
    assert_eq!(
        visible_step_count(&mut app),
        3,
        "exactly the three active-storey route cells are drawn (flat route, no link marker)",
    );
    // A cell NOT in the route has no drawn step over it.
    assert!(
        !step_visible_at(&mut app, off_route),
        "a cell NOT in the route (40,40,L0) has no drawn step (off-route is dark)",
    );
}

/// C5 (HARD CUT) — an off-`ActiveLevel` route cell is NOT drawn on the active storey, and the
/// route leaving the storey draws a MINIMAL marker at the last active-storey cell (so the
/// active-storey steps + one link marker are drawn, the off-storey cells hard-cut).
#[test]
fn off_storey_route_cells_hard_cut() {
    let mut app = preview_app();

    let l0 = Level::new(0);
    let l1 = Level::new(1);
    // The storey-1 cells sit at planar columns DISTINCT from the active-storey steps + the link
    // cell, so `step_visible_at` (planar-only) cleanly proves they are not drawn.
    let on0_a = CellLevel::new(Cell::new(5, 5), l0);
    let on0_b = CellLevel::new(Cell::new(6, 5), l0); // the last active-storey cell (link point)
    let on1_a = CellLevel::new(Cell::new(8, 5), l1);
    let on1_b = CellLevel::new(Cell::new(9, 5), l1);

    set_preview(&mut app, vec![on0_a, on0_b, on1_a, on1_b], Tu::new(20));
    assert!(
        settle_steps(&mut app),
        "the active-storey steps must have drawn"
    );

    // POSITIVE: the two storey-0 cells are drawn.
    assert!(
        step_visible_at(&mut app, on0_a),
        "storey-0 route cell (5,5,L0) drawn"
    );
    assert!(
        step_visible_at(&mut app, on0_b),
        "storey-0 route cell (6,5,L0) drawn"
    );
    // HARD CUT: the storey-1 cells are NOT drawn on the active storey — at their own planar
    // columns (8,5) / (9,5), which no active-storey step or link marker occupies.
    assert!(
        !step_visible_at(&mut app, CellLevel::new(Cell::new(8, 5), l0)),
        "an off-storey route cell must NOT be drawn on the active storey (the hard cut, C5)",
    );
    assert!(
        !step_visible_at(&mut app, CellLevel::new(Cell::new(9, 5), l0)),
        "the second off-storey route cell is hard-cut too",
    );
    // Two storey-0 steps + one link marker (drawn at the last active-storey cell) = 3 visible.
    assert_eq!(
        visible_step_count(&mut app),
        3,
        "two active-storey steps + one off-storey link marker (storey-1 cells hard-cut)",
    );
}

/// C4 (CLEARS) — clearing the preview HIDES every step (mutate-not-respawn: the pooled entities
/// persist but go `Hidden`), so a target change / deselect leaves no stale route.
#[test]
fn clearing_preview_hides_all_steps() {
    let mut app = preview_app();
    let l0 = Level::new(0);
    let cell = CellLevel::new(Cell::new(7, 7), l0);

    set_preview(&mut app, vec![cell], Tu::new(4));
    assert!(settle_steps(&mut app), "the preview must have drawn");
    assert!(
        step_visible_at(&mut app, cell),
        "the route cell is drawn before clear"
    );

    // Clear (no target) — the pooled step must hide, not linger drawn.
    app.world_mut().insert_resource(PathPreview::cleared());
    app.update();

    assert!(
        !step_visible_at(&mut app, cell),
        "after the preview is cleared, no step stays drawn (mutate-not-respawn hide)",
    );
    assert_eq!(
        visible_step_count(&mut app),
        0,
        "a cleared preview leaves zero drawn steps (the pooled sprites are hidden)",
    );
}

/// C4 (target change) — a step pooled for a prior longer route is HIDDEN when the route shrinks
/// (mutate-not-respawn surplus-hide), so a shorter route never shows a leftover cell.
#[test]
fn shrinking_route_hides_surplus_pooled_steps() {
    let mut app = preview_app();
    let l0 = Level::new(0);
    let a = CellLevel::new(Cell::new(5, 5), l0);
    let b = CellLevel::new(Cell::new(6, 5), l0);
    let c = CellLevel::new(Cell::new(7, 5), l0);

    set_preview(&mut app, vec![a, b, c], Tu::new(12));
    assert!(settle_steps(&mut app), "the preview must have drawn");
    assert_eq!(visible_step_count(&mut app), 3, "all three cells drawn");

    // Shrink to one cell (a closer target) — the surplus pooled steps must hide.
    set_preview(&mut app, vec![a], Tu::new(4));
    app.update();
    assert_eq!(
        visible_step_count(&mut app),
        1,
        "the shrunk route draws exactly one cell (the surplus pooled steps are hidden)",
    );
}

/// GTW-368 C2 (COST-ON-TARGET, positive) — with a previewed route the presenter draws a SINGLE
/// `Visible` cost label at the NAMED target cell (the route's last cell) reading the route cost
/// as `"N TU"`. No per-cell labels (exactly one label entity).
#[test]
fn target_cell_cost_label_renders_at_destination() {
    let mut app = preview_app();
    let l0 = Level::new(0);
    let start = CellLevel::new(Cell::new(10, 10), l0);
    let mid = CellLevel::new(Cell::new(11, 10), l0);
    let target = CellLevel::new(Cell::new(12, 10), l0);

    set_preview(&mut app, vec![start, mid, target], Tu::new(12));
    assert!(
        settle_label(&mut app),
        "the target cost label must have drawn"
    );

    // POSITIVE: exactly ONE visible label, reading the route cost, over the NAMED target cell.
    assert_eq!(
        visible_label_count(&mut app),
        1,
        "exactly ONE cost label is drawn (on the target cell only — no per-cell labels)",
    );
    let state = target_label_state(&mut app, target);
    assert!(state.is_some(), "the single target cost label must exist");
    let (text, over_target, visible) = state.unwrap_or_default();
    assert!(
        visible,
        "the target cost label is Visible while a route is previewed"
    );
    assert!(
        over_target,
        "the cost label is positioned ABOVE the NAMED target cell (12,10,L0)",
    );
    assert_eq!(
        text, "12 TU",
        "the target cost label reads the route cost (PathPreview::cost) as \"12 TU\"",
    );
}

/// GTW-368 C2 (HARD CUT) — a target cell on a DIFFERENT storey is NOT labelled on the active
/// storey (the label hard-cuts to the active storey like the route steps).
#[test]
fn target_label_hard_cut_when_target_off_storey() {
    let mut app = preview_app();
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let on0 = CellLevel::new(Cell::new(5, 5), l0);
    // The route climbs to storey 1; the destination (target) is off the active storey.
    let target = CellLevel::new(Cell::new(8, 5), l1);

    set_preview(&mut app, vec![on0, target], Tu::new(14));
    // The active-storey step renders, so settle on the step pool.
    assert!(
        settle_steps(&mut app),
        "the active-storey step must have drawn"
    );

    assert_eq!(
        visible_label_count(&mut app),
        0,
        "no cost label is shown when the target cell is off the active storey (the hard cut)",
    );
}

/// GTW-368 C2 / C4 (CLEARS + idle board) — clearing the preview HIDES the cost label (and every
/// step), and the IDLE board (no target) has NO visible overlay entity (no per-cell labels — the
/// GTW-357 reachable overlay was removed entirely; the only path-preview entities that can exist
/// are the pooled step sprites + the single cost label, and both are hidden with no target).
#[test]
fn clearing_preview_hides_cost_label_and_idle_board_is_clean() {
    let mut app = preview_app();
    let l0 = Level::new(0);
    let target = CellLevel::new(Cell::new(7, 7), l0);

    set_preview(
        &mut app,
        vec![CellLevel::new(Cell::new(6, 7), l0), target],
        Tu::new(8),
    );
    assert!(settle_label(&mut app), "the cost label must have drawn");
    assert_eq!(
        visible_label_count(&mut app),
        1,
        "the cost label is shown before clear"
    );

    // Clear (no target) — the cost label AND the steps must hide, not linger drawn.
    app.world_mut().insert_resource(PathPreview::cleared());
    app.update();

    // The IDLE board: zero visible steps AND zero visible labels (a clean board, no overlay).
    assert_eq!(
        visible_label_count(&mut app),
        0,
        "after the preview is cleared, the cost label hides (mutate-not-respawn)",
    );
    assert_eq!(
        visible_step_count(&mut app),
        0,
        "a cleared preview leaves zero drawn steps — the idle board is clean (no overlay)",
    );
}
