//! Tests for the world->cell inverse projection (relocated from `lib.rs`, GTW-201) and the
//! GTW-300 [`InspectTarget`] reshape (its LIVE-hovered-cell + pinned-target split).

use bevy::prelude::Vec2;
use gdtf_battle_presenter::{CELL_PX, cell_to_world};
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};

use crate::picking::{
    hovered::{InspectMode, InspectTarget},
    projection::world_to_cell,
};

/// AC2 — the world->cell inverse is the documented floored inverse of
/// `cell_to_world` (a coordinate-system FACT, pinned the way the forward map's
/// `cell_to_world_projects_row_zero_to_the_top` is pinned).
///
/// For a handful of representative cells, `cell_to_world(cell, L)` then
/// `world_to_cell(that world (x,y), L)` must round-trip back to the SAME cell.
#[test]
fn world_to_cell_inverts_cell_to_world() {
    let level = Level::new(0);
    for (cx, cy) in [(0, 0), (1, 2), (12, 7), (59, 59), (30, 0)] {
        let cell = Cell::new(cx, cy);
        let world = cell_to_world(cell, level);
        let back = world_to_cell(world.truncate(), level);
        assert_eq!(
            back,
            Some(CellLevel::new(cell, level)),
            "cell ({cx},{cy}) must round-trip through cell_to_world -> world_to_cell",
        );
    }
}

/// AC2 — a world point INSIDE a cell (not on the corner) still floors into that
/// cell: a point 0.5 cell past the corner maps to the corner's cell, proving
/// the inverse FLOORS (never rounds).
#[test]
fn world_to_cell_floors_within_a_cell() {
    let level = Level::new(0);
    // The interior of cell (3, 4): half a cell past its corner on each axis.
    // x = (3 + 0.5) * CELL_PX ; y = -((4 + 0.5) * CELL_PX) (the negated row).
    let interior = Vec2::new(3.5 * CELL_PX, -4.5 * CELL_PX);
    assert_eq!(
        world_to_cell(interior, level),
        Some(CellLevel::new(Cell::new(3, 4), level)),
        "an interior world point must floor into its containing cell",
    );
}

/// AC3 — the inverse fails closed to `None` for an off-grid world point: a
/// negative-x cursor (left of column 0) and a beyond-row-59 cursor both yield
/// `None`.
#[test]
fn world_to_cell_off_grid_is_none() {
    let level = Level::new(0);
    // Left of column 0 (cell.x = floor(-1) = -1, outside 0..60).
    assert_eq!(
        world_to_cell(Vec2::new(-CELL_PX, 0.0), level),
        None,
        "a world point left of column 0 must be None",
    );
    // Below row 59: world.y = -(60 * CELL_PX) => cell.y = 60, outside 0..60.
    assert_eq!(
        world_to_cell(Vec2::new(0.0, -60.0 * CELL_PX), level),
        None,
        "a world point below row 59 must be None",
    );
}

/// A representative in-grid cell for the [`InspectTarget`] reshape tests.
fn sample_cell() -> CellLevel {
    CellLevel::new(Cell::new(7, 3), Level::new(0))
}

/// GTW-300 — with NO pin, the EFFECTIVE inspect mode is the LIVE hovered cell: `effective()`
/// returns `Hovered(hovered())`, so the panel behaves exactly as it did reading the cursor cell.
///
/// This is the slice-2 behavior-preserving invariant — the only mode the picker ever produces.
#[test]
fn unpinned_effective_is_the_hovered_cell() {
    let cell = sample_cell();
    let target = InspectTarget::new(Some(cell));

    assert_eq!(target.pinned(), None, "a fresh target has no pin");
    assert_eq!(target.hovered(), Some(cell), "the live hovered cell is set");
    assert_eq!(
        target.effective(),
        InspectMode::Hovered(Some(cell)),
        "with no pin, effective() is the live hovered cell",
    );
}

/// GTW-300 — a PINNED target's EFFECTIVE mode is `Pinned(cell)` REGARDLESS of the live hovered
/// cell, and the live hovered cell keeps tracking underneath (it is NOT cleared by the pin).
///
/// This proves the contract's "hover no longer changes the panel while pinned" (the panel reads
/// `effective()`, which stays `Pinned` even as the cursor moves) AND that unpin can resume hover
/// with no stale value (the hovered cell was never paused). The pin is a CELL (cover has no
/// entity), so the panel resolves it through the same occupant / terrain lookup as a hovered cell.
#[test]
fn pin_overrides_effective_but_hover_keeps_tracking() {
    let first = sample_cell();
    let pinned = CellLevel::new(Cell::new(20, 4), Level::new(0));
    let mut target = InspectTarget::new(Some(first));

    // Pin to a cell: effective() flips to Pinned, ignoring the cursor.
    target.set_pinned(pinned);
    assert_eq!(target.pinned(), Some(pinned), "the pin is recorded");
    assert_eq!(
        target.effective(),
        InspectMode::Pinned(pinned),
        "a pin makes effective() the pinned cell, not the hovered cell",
    );

    // The cursor moves to a DIFFERENT cell while pinned: the live hovered cell updates, but the
    // effective mode stays Pinned (hover no longer changes the panel while pinned).
    let moved = CellLevel::new(Cell::new(12, 9), Level::new(0));
    assert_ne!(moved, first, "the cursor moved to a new cell");
    assert_ne!(moved, pinned, "the cursor moved off the pinned cell");
    target.set_hovered(Some(moved));
    assert_eq!(
        target.hovered(),
        Some(moved),
        "the live hovered cell keeps tracking the cursor under a pin",
    );
    assert_eq!(
        target.effective(),
        InspectMode::Pinned(pinned),
        "the effective mode is still Pinned while the pin holds",
    );

    // Unpin: effective() resumes the (already-current) live hovered cell — no stale value.
    target.clear_pin();
    assert_eq!(target.pinned(), None, "the pin is cleared");
    assert_eq!(
        target.effective(),
        InspectMode::Hovered(Some(moved)),
        "unpin resumes hover with the CURRENT live cell (not the stale pre-pin cell)",
    );
}
