//! Pure-logic unit tests for the reachable-overlay read-seam + label formatting.
//!
//! The DRAW-system behaviour (the tint sprites + labels actually rendered at the right
//! cells, hard-cut to the active storey) is the headless integration proof in
//! `tests/reachable_overlay.rs` (the `fog_present.rs` pattern) and the pixel proof in
//! `tests/reachable_overlay_readback.rs` (the `fog_shader_readback.rs` pattern) — those wire
//! the REAL `ReachableOverlay` → draw systems. These cover the seam + the label format that
//! do not need an app.

use gdtf_battle_sim::{Cell, CellLevel, Level, Tu};

use super::overlay::{ReachableOverlay, label_text};

/// The cleared overlay is empty — nothing selected → nothing lit.
#[test]
fn cleared_overlay_is_empty() {
    let overlay = ReachableOverlay::cleared();
    assert!(
        overlay.is_empty(),
        "the cleared overlay must hold no reachable cells"
    );
}

/// `ReachableOverlay::new` round-trips the `(cell, cost)` pairs the sim flood returns,
/// readable through the `Deref` to the inner slice.
#[test]
fn new_overlay_holds_the_reachable_pairs() {
    let cells = vec![
        (CellLevel::new(Cell::new(3, 4), Level::new(0)), Tu::new(8)),
        (CellLevel::new(Cell::new(5, 4), Level::new(0)), Tu::new(12)),
    ];
    let overlay = ReachableOverlay::new(cells.clone());
    assert_eq!(
        overlay.len(),
        2,
        "the overlay holds one entry per reachable cell"
    );
    assert_eq!(
        &*overlay,
        &cells[..],
        "the overlay derefs to the reachable (cell, cost) slice unchanged"
    );
}

/// The TU-cost label reads the bare count followed by `" TU"` (the firemode / status-panel
/// TU convention), so the label shows the EXACT `reachable_within` cost.
#[test]
fn label_text_shows_the_tu_cost() {
    assert_eq!(
        label_text(Tu::new(0)),
        "0 TU",
        "a zero-cost cell reads 0 TU"
    );
    assert_eq!(
        label_text(Tu::new(12)),
        "12 TU",
        "a 12-TU cell reads the exact cost"
    );
}
