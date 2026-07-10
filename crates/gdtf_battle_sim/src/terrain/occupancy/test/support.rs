//! Shared grid fixtures + neighbour probes for the occupancy tests — reached by
//! each concern file via `use super::support::*;`.

use bevy::platform::collections::HashSet;

use super::super::{OccupancyGrid, TerrainKind, pathable_neighbors};
use crate::{
    ganger::Tu,
    injuries::MovementCostFactor,
    metric::{Cell, CellLevel, Level},
    terrain::floor::FloorCostGrid,
    tuning::MoveCosts,
};

/// An empty stair-cell set for tests that don't exercise stair occupancy — the
/// GTW-391 `build_from_occupancy_input` signature change passes this to keep every
/// existing test call-site unmodified in intent (no stair cells = lower-only behavior).
pub(super) fn no_stair_cells() -> HashSet<CellLevel> {
    HashSet::default()
}

pub(super) fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// The grid extent constants as `i32` cell coordinates — a checked conversion
/// (`GRID_WIDTH`/`GRID_HEIGHT` are `usize` so a raw `as i32` cast trips
/// `cast_possible_wrap`). The fallback is unreachable for the 60-cell extents but
/// keeps the test free of `unwrap`/`expect` (denied in tests too).
pub(super) fn extent_i32(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}

/// Build a fresh grid with the given `(cell, level, terrain)` placements set —
/// a HAND-BUILT fixture (C6), NOT the real asset loader. Every other slot stays
/// the default [`TerrainKind::Open`].
///
/// GTW-501: `pathable_neighbors` now reads the TAG-derived path-blocking surface
/// ([`OccupancyGrid::is_path_blocked`]), not the kind-based [`TerrainKind`] marker, so a
/// blocking placement is mirrored into that surface too — reproducing the projection a
/// `Wall`/`Cover` def's `BlocksPathfinding` marker yields, so these pre-GTW-501 neighbour
/// fixtures behave identically (the C5 zero-regression guarantee).
pub(super) fn grid_with(terrain: &[(CellLevel, TerrainKind)]) -> OccupancyGrid {
    let mut grid = OccupancyGrid::new();
    for &(at, kind) in terrain {
        grid.set_terrain(at, kind);
        if *kind.blocks() {
            grid.set_path_blocking(at);
        }
    }
    grid
}

/// The default [`FloorCostGrid`] for the occupancy tests — a uniform grid seeded from
/// the default [`MoveCosts::open`] value. Keeps the neighbour-cost tests consistent
/// with the pre-GTW-396 `MoveCosts::default()` expectations (every cell costs `open`).
pub(super) fn default_floor_costs() -> FloorCostGrid {
    FloorCostGrid::new(MoveCosts::default().open, [])
}

/// Collect `pathable_neighbors` into a `Vec` of `(cell, cost)` for assertions,
/// reducing each [`CellLevel`] to its `(x, y, level)` triple so the relations
/// read clearly. GTW-396: takes `&FloorCostGrid` instead of `MoveCosts`.
pub(super) fn neighbours_of(
    origin: CellLevel,
    grid: &OccupancyGrid,
    floor_costs: &FloorCostGrid,
) -> Vec<((i32, i32, i32), Tu)> {
    pathable_neighbors(origin, grid, floor_costs, MovementCostFactor::IDENTITY)
        .map(|(cell, cost)| ((cell.x, cell.y, cell.z), cost))
        .collect()
}

/// Whether `cell` appears among the enumerated neighbours of `origin`.
/// GTW-396: takes `&FloorCostGrid` instead of `MoveCosts`.
pub(super) fn yields_neighbour(
    origin: CellLevel,
    target: CellLevel,
    grid: &OccupancyGrid,
    floor_costs: &FloorCostGrid,
) -> bool {
    pathable_neighbors(origin, grid, floor_costs, MovementCostFactor::IDENTITY)
        .any(|(cell, _)| cell == target)
}
