//! Tests for the world->cell inverse projection (relocated from `lib.rs`, GTW-201).

use bevy::prelude::Vec2;
use gdtf_battle_presenter::{CELL_PX, cell_to_world};
use gdtf_battle_sim::{Cell, CellLevel, Level};

use crate::picking::projection::world_to_cell;

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
