//! The world->cell inverse projection (GTW-221): floors a world point into the
//! `(cell, level)` it falls in, the INVERSE of the presenter's `cell_to_world`.

use bevy::prelude::*;
use gdtf_battle_presenter::CELL_PX;
use gdtf_battle_sim::{Cell, CellLevel, GRID_HEIGHT, GRID_WIDTH, Level};

/// The INVERSE of the presenter's `cell_to_world`: a world point -> the
/// `(cell, level)` it falls in, or [`None`] when the cell is outside the 60×60
/// grid.
///
/// The forward map is `cell_to_world(cell, level) = (cell.x * CELL_PX, -(cell.y) *
/// CELL_PX, z_for(level))`, so the inverse floors the scaled world coordinates
/// (reusing [`CELL_PX`] — the presenter's SINGLE px source of truth, never a
/// recomputed `16.0`):
///
/// - `cell.x = floor(world.x / CELL_PX)`
/// - `cell.y = floor(-world.y / CELL_PX)` — the forward map negates `cell.y`, so
///   the inverse negates `world.y`.
///
/// Floored, never rounded — the metric's floor-not-round contract (mirroring
/// `pos_to_cell` in the sim). Returns [`None`] (nothing hovered) when the cell is
/// outside `0..`[`GRID_WIDTH`] × `0..`[`GRID_HEIGHT`] (= 60×60).
#[must_use]
pub fn world_to_cell(world: Vec2, level: Level) -> Option<CellLevel> {
    let cx = floor_to_cell_coord(world.x / CELL_PX);
    let cy = floor_to_cell_coord(-world.y / CELL_PX);
    if in_grid(cx, cy) {
        Some(CellLevel::new(Cell::new(cx, cy), level))
    } else {
        None
    }
}

/// Whether `(cx, cy)` is inside the 60×60 grid (`0..`[`GRID_WIDTH`] ×
/// `0..`[`GRID_HEIGHT`]).
///
/// The bounds are the sim's structural [`GRID_WIDTH`] / [`GRID_HEIGHT`] (= 60),
/// widened to `i32` so a negative (above/left of origin) coordinate is rejected
/// rather than wrapping.
fn in_grid(cx: i32, cy: i32) -> bool {
    (0..grid_extent_i32(GRID_WIDTH)).contains(&cx)
        && (0..grid_extent_i32(GRID_HEIGHT)).contains(&cy)
}

/// Widens a `usize` grid extent (`GRID_WIDTH` / `GRID_HEIGHT`) to `i32` for a cell
/// bounds check.
///
/// `as i32` on a `usize` trips `cast_possible_wrap` (`-D`); [`i32::try_from`] is the
/// no-`unwrap` cast — an unrepresentable extent saturates to [`i32::MAX`], which only
/// makes the bounds check MORE permissive on a (impossible) absurd grid, never narrower.
fn grid_extent_i32(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}

/// Floors a scaled world coordinate to its integer cell index, clamped into the
/// `i32` range so a wild (or `NaN`) coordinate can never wrap on the cast.
///
/// Mirrors the sim metric's `floor_to_i32`: [`f32::floor`] so a negative coordinate
/// goes to the lower integer (−0.5 → −1, the floor-not-round contract), then a
/// clamp into the `i32` range before the cast. A `NaN` clamps to `0` (the
/// out-of-grid path then rejects it via [`in_grid`]).
const fn floor_to_cell_coord(scaled: f32) -> i32 {
    let floored = scaled.floor();
    let clamped = floored.clamp(i32::MIN as f32, i32::MAX as f32);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped into the i32 range above, so the cast cannot wrap; the fractional part is gone after floor"
    )]
    let coord = clamped as i32;
    coord
}
