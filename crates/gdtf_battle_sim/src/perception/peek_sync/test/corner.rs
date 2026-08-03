use bevy::math::Vec2;

use super::super::corner::{PEEK_LEAN, corner_lean};
use crate::{
    los::PeekOffset,
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, TerrainKind},
};

fn key(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn grid_with_walls(walls: &[(i32, i32)]) -> OccupancyGrid {
    let mut grid = OccupancyGrid::new();
    for &(x, y) in walls {
        grid.set_terrain(key(x, y), TerrainKind::Wall);
    }
    grid
}

fn ganger() -> CellLevel {
    key(3, 3)
}

#[test]
fn wall_east_open_south_leans_south() {
    let grid = grid_with_walls(&[(4, 3), (4, 4)]);
    assert_eq!(
        corner_lean(ganger(), &grid),
        PeekOffset::new(Vec2::new(0.0, -PEEK_LEAN)),
        "a wall to the East with only its South end open must lean South (0, -0.4)"
    );
}

#[test]
fn wall_north_open_east_leans_east() {
    let grid = grid_with_walls(&[(3, 4), (2, 4)]);
    assert_eq!(
        corner_lean(ganger(), &grid),
        PeekOffset::new(Vec2::new(PEEK_LEAN, 0.0)),
        "a wall to the North with only its East end open must lean East (0.4, 0)"
    );
}

#[test]
fn wall_south_open_west_leans_west() {
    let grid = grid_with_walls(&[(3, 2), (4, 2)]);
    assert_eq!(
        corner_lean(ganger(), &grid),
        PeekOffset::new(Vec2::new(-PEEK_LEAN, 0.0)),
        "a wall to the South with only its West end open must lean West (-0.4, 0)"
    );
}

#[test]
fn wall_west_open_north_leans_north() {
    let grid = grid_with_walls(&[(2, 3), (2, 2)]);
    assert_eq!(
        corner_lean(ganger(), &grid),
        PeekOffset::new(Vec2::new(0.0, PEEK_LEAN)),
        "a wall to the West with only its North end open must lean North (0, 0.4)"
    );
}

#[test]
fn l_inside_corner_leans_past_open_end() {
    let grid = grid_with_walls(&[(3, 4), (4, 4), (4, 3)]);
    assert_eq!(
        corner_lean(ganger(), &grid),
        PeekOffset::new(Vec2::new(-PEEK_LEAN, 0.0)),
        "an L inside-corner must lean past the first blocked cardinal's open end (West), \
         never toward the other (East) wall"
    );
}

#[test]
fn long_flat_wall_no_peek() {
    let grid = grid_with_walls(&[(4, 2), (4, 3), (4, 4)]);
    assert_eq!(
        corner_lean(ganger(), &grid),
        PeekOffset::default(),
        "a long flat wall (both ends blocked) offers no corner to lean around"
    );
}

#[test]
fn lone_pillar_no_peek() {
    let grid = grid_with_walls(&[(4, 3)]);
    assert_eq!(
        corner_lean(ganger(), &grid),
        PeekOffset::default(),
        "a lone pillar (both ends open) yields no positional lean (deferred to a target-aware consumer)"
    );
}

#[test]
fn surrounded_no_peek() {
    let grid = grid_with_walls(&[
        (3, 4),
        (4, 4),
        (4, 3),
        (4, 2),
        (3, 2),
        (2, 2),
        (2, 3),
        (2, 4),
    ]);
    assert_eq!(
        corner_lean(ganger(), &grid),
        PeekOffset::default(),
        "a fully surrounded cell has no open corner edge → no peek"
    );
}

#[test]
fn open_ground_no_peek() {
    let grid = grid_with_walls(&[]);
    assert_eq!(
        corner_lean(ganger(), &grid),
        PeekOffset::default(),
        "a ganger on open ground hugs no corner → no peek"
    );
}
