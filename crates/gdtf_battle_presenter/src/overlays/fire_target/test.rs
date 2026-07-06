//! Pure-logic unit tests for the fire-target highlight read-seam + the cost-label text.
//!
//! The DRAW-system behaviour (the red tile rendered UNDER the actor at the named cell, the
//! opaque cost label, hard-cut to the active storey, hidden off a fireable hover) is the
//! headless integration proof in `tests/fire_target.rs` (the `path_preview.rs` pattern) — that
//! wires the REAL `FireTargetHighlight` → draw system. These cover the read-seam accessors + the
//! cost-label format that do not need an app.

use gdtf_battle_sim::prelude::{Cell, CellLevel, Level, Tu};

use super::draw::{FireTargetHighlight, cost_label_text};

/// A level-0 cell at `(x, y)`.
fn c0(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// The cleared highlight is empty — no fireable hover → nothing drawn.
#[test]
fn cleared_highlight_is_empty() {
    let highlight = FireTargetHighlight::cleared();
    assert!(
        highlight.is_empty(),
        "the cleared highlight holds no target"
    );
    assert_eq!(highlight.cell(), None, "no cell when cleared");
    assert_eq!(highlight.cost(), None, "no cost when cleared");
}

/// The `Default` highlight is the empty (cleared) one — a battle with no fireable hover draws
/// nothing.
#[test]
fn default_highlight_is_cleared() {
    assert_eq!(
        FireTargetHighlight::default(),
        FireTargetHighlight::cleared(),
        "the Default highlight is the empty one (no fireable hover)",
    );
}

/// `FireTargetHighlight::new` round-trips the hovered cell + the fire cost, readable through the
/// accessors.
#[test]
fn new_highlight_holds_cell_and_cost() {
    let cell = c0(12, 8);
    let highlight = FireTargetHighlight::new(cell, Tu::new(14));
    assert!(!highlight.is_empty(), "a populated highlight is non-empty");
    assert_eq!(
        highlight.cell(),
        Some(cell),
        "the highlight holds the hovered fireable-enemy cell",
    );
    assert_eq!(
        highlight.cost(),
        Some(Tu::new(14)),
        "the highlight exposes the fire TU cost (mode_tu_cost) unchanged",
    );
}

/// The fire-cost label reads the bare TU count followed by `" TU"` (the firemode / move-cost
/// convention), so the label shows the EXACT fire cost.
#[test]
fn cost_label_text_shows_the_tu_cost() {
    assert_eq!(
        cost_label_text(Tu::new(0)),
        "0 TU",
        "a zero cost reads 0 TU"
    );
    assert_eq!(
        cost_label_text(Tu::new(14)),
        "14 TU",
        "a 14-TU fire cost reads the exact cost",
    );
}
