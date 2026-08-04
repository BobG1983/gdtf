use bevy::math::Vec2;

use crate::{
    los::PeekOffset,
    metric::{Cell, CellLevel},
    occupancy::OccupancyGrid,
};

const CARDINALS: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];

pub(super) const PEEK_LEAN: f32 = 0.4;

pub(super) fn corner_lean(cell: CellLevel, grid: &OccupancyGrid) -> PeekOffset {
    for (dx, dy) in CARDINALS {
        let wall = offset_in_plane(cell, Cell::new(dx, dy));
        if !*grid.is_blocked(&wall) {
            continue;
        }
        let (px, py) = (-dy, dx);
        let plus_open = !*grid.is_blocked(&offset_in_plane(wall, Cell::new(px, py)));
        let minus_open = !*grid.is_blocked(&offset_in_plane(wall, Cell::new(-px, -py)));
        if plus_open != minus_open {
            let (lx, ly) = if plus_open { (px, py) } else { (-px, -py) };
            return PeekOffset::new(Vec2::new(lx as f32, ly as f32) * PEEK_LEAN);
        }
    }
    PeekOffset::default()
}

fn offset_in_plane(at: CellLevel, delta: Cell) -> CellLevel {
    CellLevel::new(Cell::new(at.x + delta.x, at.y + delta.y), at.level())
}
