//! Relocated unit tests for the faced-cell helper (GTW-201 wave 22 — moved verbatim
//! from the former inline `#[cfg(test)] mod tests`).

use crate::{
    faced_cell::faced_cell,
    ganger::{Direction, Facing, Position},
    metric::{Cell, CellLevel, Level},
};

/// Build a shooter position at the given cell coords on the given storey.
fn shooter_at(x: i32, y: i32, storey: u8) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), Level::new(storey)))
}

// --- AC1: all 8 directions advance by that direction's signed unit step on each
// axis it spans, same level. Parametrized sweep with EXACT expected cells — this is
// coordinate-system geometry, so exact assertions are correct (not tuning).

/// Every [`Direction`] faces the shooter cell plus its signed unit step, on the
/// shooter's own storey. The diagonals (NE/SE/SW/NW) advance by one cell on BOTH
/// axes — the case the center-offset + floor derivation must get right.
#[test]
fn all_eight_directions_advance_by_signed_unit_step() {
    // North is −Y, East is +X (ganger.rs Direction orientation). Each diagonal
    // combines its two cardinals' signs by exactly one cell.
    // (direction, dx, dy) — the integer signed step the faced cell must add.
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

// --- AC2: a NE-facing shooter at (10,10) faces (11,9) — proving the center-offset
// derivation. A corner derivation (0.0 + 0.707 → floor 0) would return (10,10) and
// fail; the center (0.5 + 0.707 = 1.207 → floor 1) crosses the boundary.

/// A NorthEast-facing shooter at `(10, 10, L)` faces `(11, 9, L)` — the diagonal
/// crosses one cell on each axis only because the step leaves from the cell center.
#[test]
fn northeast_diagonal_crosses_from_cell_center_not_corner() {
    let level = 2u8;
    let pos = shooter_at(10, 10, level);
    let facing = Facing::new(Direction::NorthEast);
    let (faced, faced_level) = faced_cell(&pos, &facing);
    // NE is +X, −Y, so (10,10) → (11, 9). A corner derivation would (wrongly)
    // return (10, 10).
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

// --- AC3: the returned Level equals the shooter's level unchanged, for a non-zero
// level (the brace cell is on the shooter's own storey; the step is horizontal).

/// The faced cell keeps the shooter's storey for a non-zero level — the forward
/// step is horizontal (`z = 0`), so it can never change the level.
#[test]
fn level_is_preserved_for_a_non_zero_level() {
    let level = 5u8;
    let pos = shooter_at(7, 7, level);
    // Sweep every direction so no facing leaks the level off the shooter's storey.
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

// --- AC4: zero-pixel, floor-not-round contract at a negative cell. A West-facing
// shooter at (0, y) faces (-1, y): center 0.5 + (−1.0) = −0.5 → floor −1 (a round
// would give 0 and fail to advance). It composes only metric.rs sim-unit
// conversions — no pixel anywhere.

/// A West-facing shooter at `(0, y)` faces `(-1, y)` — the floor-not-round contract
/// advances correctly into a negative cell. (Sim-unit coords only; zero pixels.)
#[test]
fn west_facing_at_zero_floors_to_negative_cell() {
    let y = 4;
    let level = 1u8;
    let pos = shooter_at(0, y, level);
    let facing = Facing::new(Direction::West);
    let (faced, faced_level) = faced_cell(&pos, &facing);
    // center.x = 0.5, step.x = −1.0 → 0.5 + (−1.0) = −0.5 → floor −1. A round-based
    // derivation would give 0 and fail to leave the shooter's cell.
    assert_eq!(
        faced,
        Cell::new(-1, y),
        "West from (0,y) must floor to (-1,y), not round to (0,y)",
    );
    assert_eq!(faced_level, Level::new(level));
}
