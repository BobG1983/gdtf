//! World-to-cell projection for pointer picking.

use bevy::prelude::*;
use gdtf_battle_presenter::CELL_PX;
use gdtf_battle_sim::{
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    prelude::{Cell, CellLevel, Level},
};

/// Map a world XY position to a cell on `level`, if in grid bounds.
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

fn in_grid(cx: i32, cy: i32) -> bool {
    (0..grid_extent_i32(GRID_WIDTH)).contains(&cx)
        && (0..grid_extent_i32(GRID_HEIGHT)).contains(&cy)
}

fn grid_extent_i32(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}

const fn floor_to_cell_coord(scaled: f32) -> i32 {
    let floored = scaled.floor();
    let clamped = floored.clamp(i32::MIN as f32, i32::MAX as f32);
    let coord = clamped as i32;
    coord
}
