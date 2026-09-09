use gdtf_battle_presenter::PathPreview;
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level, Tu};

use super::harness::*;

#[test]
fn target_cell_cost_label_renders_at_destination() {
    let mut app = preview_app();
    let l0 = Level::new(0);
    let start = CellLevel::new(Cell::new(10, 10), l0);
    let mid = CellLevel::new(Cell::new(11, 10), l0);
    let target = CellLevel::new(Cell::new(12, 10), l0);

    set_preview(&mut app, vec![start, mid, target], Tu::new(12));
    settle_label(&mut app);

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

#[test]
fn target_label_hard_cut_when_target_off_storey() {
    let mut app = preview_app();
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let on0 = CellLevel::new(Cell::new(5, 5), l0);
    let target = CellLevel::new(Cell::new(8, 5), l1);

    set_preview(&mut app, vec![on0, target], Tu::new(14));
    settle_steps(&mut app);

    assert_eq!(
        visible_label_count(&mut app),
        0,
        "no cost label is shown when the target cell is off the active storey (the hard cut)",
    );
}

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
    settle_label(&mut app);
    assert_eq!(
        visible_label_count(&mut app),
        1,
        "the cost label is shown before clear"
    );

    app.world_mut().insert_resource(PathPreview::cleared());
    app.update();

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
