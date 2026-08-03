//! from the former inline `#[cfg(test)] mod tests`).

use crate::{
    faced_cell::faced_cell,
    ganger::{Direction, Facing, Position},
    metric::{Cell, CellLevel, Level},
};

fn shooter_at(x: i32, y: i32, storey: u8) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), Level::new(storey)))
}


#[test]
fn all_eight_directions_advance_by_signed_unit_step() {
    let cases = [
        (Direction::North, 0, -1),
        (Direction::NorthEast, 1, -1),
        (Direction::East, 1, 0),
        (Direction::SouthEast, 1, 1),
        (Direction::South, 0, 1),
        (Direction::SouthWest, -1, 1),
        (Direction::West, -1, 0),
        (Direction::NorthWest, -1, -1),
    ];

    let (sx, sy, storey) = (20, 20, 3u8);
    let pos = shooter_at(sx, sy, storey);
    for (dir, dx, dy) in cases {
        let facing = Facing::new(dir);
        let (faced, level) = faced_cell(&pos, &facing);
        assert_eq!(
            faced,
            Cell::new(sx + dx, sy + dy),
            "{dir:?}: faced cell must be the shooter cell plus its signed unit step",
        );
        assert_eq!(
            level,
            Level::new(storey),
            "{dir:?}: the faced cell stays on the shooter's storey",
        );
    }
}


#[test]
fn northeast_diagonal_crosses_from_cell_center_not_corner() {
    let level = 2u8;
    let pos = shooter_at(10, 10, level);
    let facing = Facing::new(Direction::NorthEast);
    let (faced, faced_level) = faced_cell(&pos, &facing);
    assert_eq!(
        faced,
        Cell::new(11, 9),
        "NE from (10,10) must face (11,9) — the center-offset derivation",
    );
    assert_ne!(
        faced,
        Cell::new(10, 10),
        "a corner derivation would fail to advance the diagonal",
    );
    assert_eq!(faced_level, Level::new(level));
}


#[test]
fn level_is_preserved_for_a_non_zero_level() {
    let level = 5u8;
    let pos = shooter_at(7, 7, level);
    for dir in [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ] {
        let (_faced, faced_level) = faced_cell(&pos, &Facing::new(dir));
        assert_eq!(
            faced_level,
            Level::new(level),
            "{dir:?}: the faced cell must stay on the shooter's storey {level}",
        );
    }
}


#[test]
fn west_facing_at_zero_floors_to_negative_cell() {
    let y = 4;
    let level = 1u8;
    let pos = shooter_at(0, y, level);
    let facing = Facing::new(Direction::West);
    let (faced, faced_level) = faced_cell(&pos, &facing);
    assert_eq!(
        faced,
        Cell::new(-1, y),
        "West from (0,y) must floor to (-1,y), not round to (0,y)",
    );
    assert_eq!(faced_level, Level::new(level));
}
