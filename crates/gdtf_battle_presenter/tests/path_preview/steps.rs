use gdtf_battle_presenter::PathPreview;
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level, Tu};

use super::harness::*;

#[test]
fn route_cells_drawn_nonroute_cell_dark() {
    let mut app = preview_app();

    let l0 = Level::new(0);
    let a = CellLevel::new(Cell::new(10, 10), l0);
    let b = CellLevel::new(Cell::new(11, 10), l0);
    let c = CellLevel::new(Cell::new(12, 10), l0);
    let off_route = CellLevel::new(Cell::new(40, 40), l0);

    set_preview(&mut app, vec![a, b, c], Tu::new(12));
    assert!(
        settle_steps(&mut app),
        "the route step sprites must have drawn"
    );

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
    assert_eq!(
        visible_step_count(&mut app),
        3,
        "exactly the three active-storey route cells are drawn (flat route, no link marker)",
    );
    assert!(
        !step_visible_at(&mut app, off_route),
        "a cell NOT in the route (40,40,L0) has no drawn step (off-route is dark)",
    );
}

#[test]
fn off_storey_route_cells_hard_cut() {
    let mut app = preview_app();

    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let on0_a = CellLevel::new(Cell::new(5, 5), l0);
    let on0_b = CellLevel::new(Cell::new(6, 5), l0); 
    let on1_a = CellLevel::new(Cell::new(8, 5), l1);
    let on1_b = CellLevel::new(Cell::new(9, 5), l1);

    set_preview(&mut app, vec![on0_a, on0_b, on1_a, on1_b], Tu::new(20));
    assert!(
        settle_steps(&mut app),
        "the active-storey steps must have drawn"
    );

    assert!(
        step_visible_at(&mut app, on0_a),
        "storey-0 route cell (5,5,L0) drawn"
    );
    assert!(
        step_visible_at(&mut app, on0_b),
        "storey-0 route cell (6,5,L0) drawn"
    );
    assert!(
        !step_visible_at(&mut app, CellLevel::new(Cell::new(8, 5), l0)),
        "an off-storey route cell must NOT be drawn on the active storey (the hard cut, C5)",
    );
    assert!(
        !step_visible_at(&mut app, CellLevel::new(Cell::new(9, 5), l0)),
        "the second off-storey route cell is hard-cut too",
    );
    assert_eq!(
        visible_step_count(&mut app),
        3,
        "two active-storey steps + one off-storey link marker (storey-1 cells hard-cut)",
    );
}

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

    set_preview(&mut app, vec![a], Tu::new(4));
    app.update();
    assert_eq!(
        visible_step_count(&mut app),
        1,
        "the shrunk route draws exactly one cell (the surplus pooled steps are hidden)",
    );
}
