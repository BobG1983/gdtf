//! Pure-logic unit tests for the path-preview read-seam + the `preview_draws` resolution.
//!
//! The DRAW-system behaviour (the step sprites actually rendered at the right cells,
//! hard-cut to the active storey, §53-dimmed on EXPLORED) is the headless integration proof
//! in `tests/path_preview.rs` (the `fog_present.rs` pattern) and the pixel proof in
//! `tests/path_preview_readback.rs` (the `fog_shader_readback.rs` pattern) — those wire the
//! REAL `PathPreview` → draw system. These cover the read-seam + the `preview_draws` pure
//! decision (§53 + hard-cut + C5 link marker) that do not need an app.

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

/// A `SquadVisibility` with `visible` cells VISIBLE and `explored` cells EXPLORED-only.
fn fog(visible: &[CellLevel], explored_only: &[CellLevel]) -> SquadVisibility {
    let vis: HashSet<CellLevel> = visible.iter().copied().collect();
    let mut exp = vis.clone();
    exp.extend(explored_only.iter().copied());
    SquadVisibility::new(vis, exp)
}

/// A level-0 cell at `(x, y)`.
fn c0(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A level-1 cell at `(x, y)`.
fn c1(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(1))
}

/// The cleared preview is empty — no target / no route → nothing drawn.
#[test]
fn cleared_preview_is_empty() {
    let preview = PathPreview::cleared();
    assert!(
        preview.is_empty(),
        "the cleared preview holds no route cells"
    );
    assert_eq!(*preview.cost(), 0, "the cleared preview has zero cost");
}

/// `PathPreview::new` round-trips the route cells + the §48 total cost, readable through the
/// accessors.
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
        "the preview exposes the §48 total cost (Path::total) unchanged",
    );
}

/// C1 / C3 — every active-storey route cell draws, and a squad-VISIBLE step is FULL alpha
/// while an EXPLORED-but-not-VISIBLE step is REDUCED alpha (the §53 remembered treatment).
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

    // Both steps must be present (no `unwrap`/`panic` in tests — the denied-lints house rule).
    assert!(
        vis_alpha.is_some() && exp_alpha.is_some(),
        "both the VISIBLE and the EXPLORED step must be drawn",
    );
    assert!(
        exp_alpha < vis_alpha,
        "the EXPLORED (remembered) step draws at a REDUCED alpha vs the VISIBLE step (§53): \
         explored={exp_alpha:?} visible={vis_alpha:?}",
    );
}

/// GTW-371 C1 (pin-discriminating) — the VISIBLE route TILE draws at EXACTLY 0.5 alpha (the
/// transparency the user asked for on the path tiles) while the cost-LABEL colour stays FULLY
/// OPAQUE (alpha 1.0, unchanged). The two are distinct: transparency on the route TILES only,
/// never on the cost TEXT.
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
        "the VISIBLE route TILE draws at EXACTLY 0.5 alpha (GTW-371 C1): the route tiles are \
         ~half transparent; got {tile_alpha:?}",
    );

    // The cost LABEL colour is FULLY OPAQUE — transparency applies to the TILE, never the TEXT.
    assert!(
        (LABEL_COLOR.alpha() - 1.0).abs() < 0.001,
        "the cost LABEL colour stays FULLY OPAQUE (alpha 1.0) — transparency is on the route \
         tiles only, never the cost text (GTW-371 C1); got {}",
        LABEL_COLOR.alpha(),
    );
}

/// C5 — an off-storey route cell is NOT drawn (the hard cut), and the route leaving the
/// active storey through a vertical link draws ONE extra MINIMAL marker at the last
/// active-storey cell (the GTW-359 soft-dep placeholder).
#[test]
fn off_storey_steps_hard_cut_with_link_marker() {
    // A route that runs across storey 0 then climbs to storey 1.
    let last_active = c0(8, 8);
    let preview = PathPreview::new(vec![c0(7, 8), last_active, c1(8, 8), c1(9, 8)], Tu::new(20));
    // Whole grid visible so §53 is not the variable under test here.
    let all_vis: Vec<CellLevel> = vec![c0(7, 8), last_active, c1(8, 8), c1(9, 8)];
    let squad = fog(&all_vis, &[]);

    let draws = preview_draws(&preview, Level::new(0), &squad);

    // The two storey-0 cells are drawn as steps; the two storey-1 cells are hard-cut; ONE
    // link marker is appended at the LAST active-storey cell (so 2 steps + 1 marker = 3).
    assert_eq!(
        draws.len(),
        3,
        "two active-storey steps + one off-storey link marker (storey-1 cells hard-cut)",
    );
    // No drawn cell is off-storey.
    assert!(
        draws.iter().all(|d| d.cell.z == 0),
        "no off-storey cell is drawn (the hard cut, C5)",
    );
    // The link marker sits at the last active-storey cell before the route departs.
    assert!(
        draws.iter().filter(|d| d.cell == last_active).count() == 2,
        "the link marker is drawn at the last active-storey cell ({last_active:?}) — its step \
         sprite plus the off-storey-continuation marker",
    );
}

/// C5 — a route that stays entirely on the active storey draws NO link marker.
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

/// An empty preview resolves to no draws (cleared / unreachable → nothing).
#[test]
fn empty_preview_resolves_to_no_draws() {
    let preview = PathPreview::cleared();
    let squad = fog(&[], &[]);
    let draws = preview_draws(&preview, Level::new(0), &squad);
    assert!(draws.is_empty(), "an empty preview draws nothing");
}

/// GTW-368 (C2) — the target-cell cost label reads the bare TU count followed by `" TU"` (the
/// firemode / status-panel TU convention), so the label shows the EXACT previewed `cost()`.
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

/// GTW-368 (C2) — the previewed TARGET cell is the LAST cell of the route, so the cost label is
/// keyed to the route destination, and the previewed cost is exposed verbatim for the label.
#[test]
fn target_cell_is_the_route_destination() {
    let start = c0(3, 4);
    let mid = c0(4, 4);
    let goal = c0(5, 4);
    let preview = PathPreview::new(vec![start, mid, goal], Tu::new(16));

    assert_eq!(
        preview.cells().last().copied(),
        Some(goal),
        "the previewed target cell is the route's last cell (the destination)",
    );
    assert_eq!(
        label_text(preview.cost()),
        "16 TU",
        "the target-cell label shows the previewed route cost (Path::total)",
    );
}
