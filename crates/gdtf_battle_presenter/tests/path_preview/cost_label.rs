//! The cost-on-target label: renders at the destination, the off-storey hard cut,
//! and clears + the idle board clean (GTW-368 C2/C4).

use gdtf_battle_presenter::PathPreview;
use gdtf_battle_sim::{Cell, CellLevel, Level, Tu};

use super::harness::*;

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
