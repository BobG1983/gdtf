//! Route step sprites: draw at named cells, the off-storey hard cut, clears, and
//! the pooled shrink (C1/C5/C4).

use gdtf_battle_presenter::PathPreview;
use gdtf_battle_sim::{Cell, CellLevel, Level, Tu};

use super::harness::*;

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
