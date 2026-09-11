use bevy::platform::collections::HashSet;

use super::super::{OccupancyGrid, TerrainKind, pathable_neighbors};
use crate::{
    ganger::Tu,
    injuries::MovementCostFactor,
    metric::{Cell, CellLevel, Level},
    terrain::floor::FloorCostGrid,
    tuning::MoveCosts,
};

pub(super) fn no_stair_cells() -> HashSet<CellLevel> {
    HashSet::default()
}

pub(super) fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

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

pub(super) fn default_floor_costs() -> FloorCostGrid {
    FloorCostGrid::new(MoveCosts::default().open, [])
}

pub(super) fn neighbours_of(
    origin: CellLevel,
    grid: &OccupancyGrid,
    floor_costs: &FloorCostGrid,
) -> Vec<((i32, i32, i32), Tu)> {
    pathable_neighbors(origin, grid, floor_costs, MovementCostFactor::IDENTITY)
        .map(|(cell, cost)| ((cell.x, cell.y, cell.z), cost))
        .collect()
}

pub(super) fn yields_neighbour(
    origin: CellLevel,
    target: CellLevel,
    grid: &OccupancyGrid,
    floor_costs: &FloorCostGrid,
) -> bool {
    pathable_neighbors(origin, grid, floor_costs, MovementCostFactor::IDENTITY)
        .any(|(cell, _)| cell == target)
}
