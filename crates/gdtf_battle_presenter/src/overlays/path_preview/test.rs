use bevy::{platform::collections::HashSet, prelude::Alpha};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level, Tu},
    visibility::SquadVisibility,
};

use super::{
    draw::{LABEL_COLOR, label_text},
    resolve::preview_draws,
    seam::PathPreview,
};

fn fog(visible: &[CellLevel], explored_only: &[CellLevel]) -> SquadVisibility {
    let vis: HashSet<CellLevel> = visible.iter().copied().collect();
    let mut exp = vis.clone();
    exp.extend(explored_only.iter().copied());
    SquadVisibility::new(vis, exp)
}

fn c0(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn c1(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(1))
}

#[test]
fn cleared_preview_is_empty() {
    let preview = PathPreview::cleared();
    assert!(
        preview.is_empty(),
        "the cleared preview holds no route cells"
    );
    assert_eq!(*preview.cost(), 0, "the cleared preview has zero cost");
}

#[test]
fn new_preview_holds_route_and_cost() {
    let cells = vec![c0(3, 4), c0(4, 4), c0(5, 4)];
    let preview = PathPreview::new(cells.clone(), Tu::new(16));
    assert_eq!(
        preview.cells(),
        &cells[..],
        "the preview holds the route cells"
    );
    assert_eq!(
        *preview.cost(),
        16,
        "the preview exposes the total cost unchanged",
    );
}

#[test]
fn visible_steps_full_alpha_explored_steps_dimmer() {
    let visible_cell = c0(5, 5);
    let explored_cell = c0(6, 5);
    let preview = PathPreview::new(vec![visible_cell, explored_cell], Tu::new(8));
    let squad = fog(&[visible_cell], &[explored_cell]);

    let draws = preview_draws(&preview, Level::new(0), &squad);
    assert_eq!(draws.len(), 2, "both active-storey route cells draw");

    let vis_alpha = draws
        .iter()
        .find(|d| d.cell == visible_cell)
        .map(|d| d.tint.alpha());
    let exp_alpha = draws
        .iter()
        .find(|d| d.cell == explored_cell)
        .map(|d| d.tint.alpha());

    assert!(
        vis_alpha.is_some() && exp_alpha.is_some(),
        "both the VISIBLE and the EXPLORED step must be drawn",
    );
    assert!(
        exp_alpha < vis_alpha,
        "the EXPLORED step draws at a reduced alpha vs the VISIBLE step: explored={exp_alpha:?} visible={vis_alpha:?}",
    );
}

#[test]
fn route_tile_is_half_alpha_but_cost_label_stays_opaque() {
    let visible_cell = c0(5, 5);
    let preview = PathPreview::new(vec![visible_cell], Tu::new(8));
    let squad = fog(&[visible_cell], &[]);

    let draws = preview_draws(&preview, Level::new(0), &squad);
    let tile_alpha = draws
        .iter()
        .find(|d| d.cell == visible_cell)
        .map(|d| d.tint.alpha());
    assert!(
        tile_alpha.is_some_and(|a| (a - 0.5).abs() < 0.001),
        "the VISIBLE route TILE draws at EXACTLY 0.5 alpha; got {tile_alpha:?}",
    );

    assert!(
        (LABEL_COLOR.alpha() - 1.0).abs() < 0.001,
        "the cost LABEL colour stays FULLY OPAQUE (alpha 1.0); got {}",
        LABEL_COLOR.alpha(),
    );
}

#[test]
fn off_storey_steps_hard_cut_with_link_marker() {
    let last_active = c0(8, 8);
    let preview = PathPreview::new(vec![c0(7, 8), last_active, c1(8, 8), c1(9, 8)], Tu::new(20));
    let all_vis: Vec<CellLevel> = vec![c0(7, 8), last_active, c1(8, 8), c1(9, 8)];
    let squad = fog(&all_vis, &[]);

    let draws = preview_draws(&preview, Level::new(0), &squad);

    assert_eq!(
        draws.len(),
        3,
        "two active-storey steps + one off-storey link marker",
    );
    assert!(
        draws.iter().all(|d| d.cell.z == 0),
        "no off-storey cell is drawn",
    );
    assert!(
        draws.iter().filter(|d| d.cell == last_active).count() == 2,
        "the link marker is drawn at the last active-storey cell",
    );
}

#[test]
fn flat_route_draws_no_link_marker() {
    let preview = PathPreview::new(vec![c0(1, 1), c0(2, 1), c0(3, 1)], Tu::new(12));
    let squad = fog(&[c0(1, 1), c0(2, 1), c0(3, 1)], &[]);

    let draws = preview_draws(&preview, Level::new(0), &squad);
    assert_eq!(
        draws.len(),
        3,
        "a flat route draws exactly its three steps and no link marker",
    );
}

#[test]
fn empty_preview_resolves_to_no_draws() {
    let preview = PathPreview::cleared();
    let squad = fog(&[], &[]);
    let draws = preview_draws(&preview, Level::new(0), &squad);
    assert!(draws.is_empty(), "an empty preview draws nothing");
}

#[test]
fn target_label_text_shows_the_tu_cost() {
    assert_eq!(
        label_text(Tu::new(0)),
        "0 TU",
        "a zero-cost route reads 0 TU"
    );
    assert_eq!(
        label_text(Tu::new(12)),
        "12 TU",
        "a 12-TU route reads the exact cost"
    );
}

#[test]
fn target_cell_is_the_route_destination() {
    let start = c0(3, 4);
    let mid = c0(4, 4);
    let goal = c0(5, 4);
    let preview = PathPreview::new(vec![start, mid, goal], Tu::new(16));

    assert_eq!(
        preview.cells().last().copied(),
        Some(goal),
        "the previewed target cell is the route's last cell",
    );
    assert_eq!(
        label_text(preview.cost()),
        "16 TU",
        "the target-cell label shows the previewed route cost",
    );
}
