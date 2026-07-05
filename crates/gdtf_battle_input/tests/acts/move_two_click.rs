//! Two-click move targeting + the cross-storey destination (GTW-356).

use bevy::prelude::*;
use gdtf_battle_input::PathPreviewTarget;
use gdtf_battle_sim::{
    CellLevel, Direction, Level, LinkKind, StanceKind, VerticalLink, acts::MoveRequested,
    build_vertical_link_graph, test_support::SituationBuilder,
};
use gdtf_test_utils::{clear_keys, clear_mouse, press_key, press_left, probed};

use super::harness::*;

/// The collected `MoveRequested` messages (GTW-356 two-click move target).
fn moves(app: &App) -> Vec<MoveRequested> {
    probed::<MoveRequested>(app)
}

/// The current `PathPreviewTarget` (the GTW-356 two-click move target the preview previews TO).
fn move_target(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<PathPreviewTarget>()
        .and_then(|t| **t)
}

// =================================================================================
// GTW-356 — two-click move targeting + cross-storey UX, over the REAL cursor ->
// InspectTarget -> left_click_act -> dispatch_act_intents chain. AC1 (target then
// commit), AC4 (cross-storey via the level keys), AC5 (default = ActiveLevel), AC6
// (a vertical-link tile is NOT a move target).
// =================================================================================

/// Presses Left + `update()`s once (resolving the click on the stable cursor cell), then
/// clears the mouse edge so the NEXT press is a fresh just-pressed — one left-click of the
/// two-click sequence. The cursor must already be set (via [`hover_at`] / a prior call).
fn click_left(app: &mut App) {
    press_left(app);
    app.update();
    clear_mouse(app);
}

/// Switches the presenter [`ActiveLevel`] UP one storey via the REAL level key (`PageUp` ->
/// `ActIntent::LevelUp` -> `dispatch_act_intents` mutates `ActiveLevel`), returning the
/// resulting active storey index. Drives the genuine keyboard -> intent -> level-step path,
/// not an injected level (GTW-356 cross-storey uses the EXISTING level switch — no new
/// mechanism).
fn switch_level_up(app: &mut App) -> i32 {
    press_key(app, test_keybinds().level_up());
    app.update();
    clear_keys(app);
    active_storey(app)
}

/// AC1 — with a player ganger selected, a FIRST left-click on a VALID move target SETS
/// `PathPreviewTarget` and emits NO `MoveRequested`; a SECOND left-click on the SAME cell
/// commits exactly one `MoveRequested { shooter, target }`, over the REAL cursor ->
/// `InspectTarget` -> `left_click_act` two-click chain.
#[test]
fn two_click_sets_target_then_commits_move() {
    let mut app = acts_app();
    add_probes(&mut app);
    // A player ganger with NO firing components fails FIRE closed, so a click on an empty cell
    // is a MOVE (the `move_path_app` precedent). Select it over the real cursor chain.
    let ganger = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, ganger);

    // Hover a DISTINCT, empty, in-grid target cell (not the shooter's own cell).
    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);

    // Click-1: SET the target, dispatch NOTHING.
    click_left(&mut app);
    assert!(
        moves(&app).is_empty(),
        "click-1 on a valid target must emit NO MoveRequested (it only sets the target)",
    );
    assert_eq!(
        move_target(&app),
        Some(target),
        "click-1 must SET PathPreviewTarget to the clicked cell",
    );

    // Click-2 on the SAME cell (cursor unmoved): COMMIT.
    click_left(&mut app);
    let emitted = moves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "click-2 on the SAME cell must emit exactly one MoveRequested (commit)",
    );
    assert_eq!(emitted[0].actor, ganger, "the move actor = the selection");
    assert_eq!(emitted[0].dest, target, "the move dest = the targeted cell");
    assert_eq!(
        move_target(&app),
        None,
        "committing the move must CLEAR PathPreviewTarget",
    );
}

/// AC5 — with NO level switch, the two-click move's `dest.z` is the DEFAULT `ActiveLevel`
/// (storey 0): the committed `MoveRequested.dest.level` equals the active level the cursor
/// resolves through `world_to_cell`.
#[test]
fn two_click_default_targets_active_level() {
    let mut app = acts_app();
    add_probes(&mut app);
    let ganger = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, ganger);

    let active = active_storey(&app);
    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    assert_eq!(
        target.z, active,
        "with no switch the hovered cell resolves at the DEFAULT active storey",
    );

    click_left(&mut app); // click-1: set target.
    click_left(&mut app); // click-2: commit.
    let emitted = moves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "the same-cell second click commits a move"
    );
    assert_eq!(
        emitted[0].dest.z, active,
        "the default move dest.z is the active storey (no switch)",
    );
}

/// AC4 — after switching `ActiveLevel` UP one storey via the EXISTING level key, the two-click
/// move on a destination-storey tile yields a `MoveRequested` whose `dest.z` == the switched
/// storey (the cross-storey encoding already flows through `world_to_cell`'s `ActiveLevel`; no
/// new level-switch, no 3D message extension).
#[test]
fn two_click_after_level_switch_targets_switched_storey() {
    let mut app = acts_app();
    add_probes(&mut app);
    let ganger = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, ganger);

    // Switch UP one storey via the real PageUp key -> ActIntent::LevelUp -> ActiveLevel.
    let switched = switch_level_up(&mut app);
    assert_ne!(
        switched, 0,
        "the level key must raise ActiveLevel off storey 0",
    );

    // Now the cursor resolves the target cell at the SWITCHED storey.
    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    assert_eq!(
        target.z, switched,
        "the hovered cell resolves at the switched active storey",
    );

    click_left(&mut app); // click-1: set target on the switched storey.
    click_left(&mut app); // click-2: commit.
    let emitted = moves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "the same-cell second click commits a move"
    );
    assert_eq!(
        emitted[0].dest.z, switched,
        "the committed move dest.z == the SWITCHED storey (cross-storey already flows)",
    );
}

/// AC6 — a left-click on a VERTICAL-LINK tile is NOT a targeting action: it emits NO
/// `MoveRequested` with the link tile as dest AND sets NO `PathPreviewTarget` (GTW-356 OQ-4).
/// Two clicks on the link tile confirm it never commits.
#[test]
fn click_on_a_link_tile_is_not_a_move_target() {
    let mut app = acts_app();
    add_probes(&mut app);
    let ganger = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, ganger);

    // Resolve the target cell, then author a vertical-link DEPARTING it (a stair up one storey),
    // so `VerticalLinkGraph::links_from(target)` reports it as a link tile. The active level is
    // storey 0 here (no switch), so the up endpoint is storey 1.
    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    let up_storey = (*target.level()).saturating_add(1);
    let up = CellLevel::new(target.cell(), Level::new(up_storey));
    let link = VerticalLink::new(target, up, LinkKind::stair());
    let graph = SituationBuilder::new()
        .slab_at(target)
        .slab_at(up)
        .vertical_link(link)
        .build();
    if let Ok(graph) = build_vertical_link_graph(&graph) {
        app.world_mut().insert_resource(graph);
    }

    // Two clicks on the link tile: it is a no-op each time — no target, no move.
    click_left(&mut app);
    click_left(&mut app);
    assert!(
        moves(&app).is_empty(),
        "a click on a vertical-link tile must emit NO MoveRequested (OQ-4: not a move target)",
    );
    assert_eq!(
        move_target(&app),
        None,
        "a click on a vertical-link tile must set NO PathPreviewTarget (OQ-4: not a target)",
    );
}
