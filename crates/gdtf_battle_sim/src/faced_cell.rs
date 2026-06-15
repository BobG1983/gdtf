//! The faced-cell geometry helper — the cell a shooter's [`Facing`] looks at.
//!
//! The §1a brace gate (`docs/combat/resolution.md` line 26: "+30 when **the faced
//! cell's** cover height suits the stance") needs the cell whose cover height the
//! stability composer (E4.3) reads. That is the ground cell one step along the
//! shooter's facing, on the shooter's **own** storey. [`faced_cell`] is that pure
//! geometry: shooter [`Position`] + [`Facing`] → the faced ([`Cell`], [`Level`]).
//! No `World` access, render-free, **zero pixels** — it composes only the
//! [`crate::metric`] sim-unit conversions (`docs/combat/battle-space.md`
//! §"Sub-cell precision on the ground plane").
//!
//! ## The diagonal subtlety
//!
//! [`Direction::forward_step`] returns a **unit** `Vec3`, so a diagonal step is
//! `(±1/√2, ±1/√2) ≈ (±0.707, ±0.707)`, not `(±1, ±1)`. Flooring that step added to
//! the bare integer cell **corner** would fail to advance a diagonal
//! (`0.0 + 0.707 = 0.707 → floor 0`). The grounded derivation steps from the cell
//! **center**: [`cell_center`] adds the `+0.5` x/y offset, so the diagonal crosses
//! the boundary (`0.5 + 0.707 = 1.207 → floor 1`) and a cardinal advances on its one
//! axis (`0.5 + 1.0 = 1.5 → floor 1`; the un-stepped axis `0.5 + 0.0 = 0.5 → floor
//! 0`). [`pos_to_cell`] floors — never rounds — so it advances correctly even at a
//! negative cell. Reusing this one metric-conversion path (rather than re-deriving
//! per-axis signs) keeps the helper aligned with the rest of the shot pipeline.

use crate::{
    ganger::{Facing, Position},
    metric::{Cell, Level, SimPos, cell_center, pos_to_cell},
};

/// The ground cell a shooter **faces** — one step along its [`Facing`], on the
/// shooter's own storey.
///
/// Given the shooter's [`Position`] (a `(cell, level)` key) and its [`Facing`],
/// returns the faced ([`Cell`], [`Level`]): the cell one unit step along the facing,
/// with the shooter's [`Level`] preserved unchanged (the step is horizontal —
/// [`Direction::forward_step`](crate::ganger::Direction::forward_step) has `z = 0` —
/// so the faced cell is on the shooter's own storey). This is the cell whose cover
/// height the §1a brace gate reads (`docs/combat/resolution.md` line 26: "+30 when
/// the faced cell's cover height suits the stance").
///
/// The faced cell is derived through the [`crate::metric`] sim-unit path so the
/// diagonal case is correct: it steps from [`cell_center`] (the `+0.5` x/y centering)
/// by the **unit** forward step, then [`pos_to_cell`] floors. A diagonal's `0.707`
/// component therefore crosses the cell boundary (`0.5 + 0.707 = 1.207 → floor 1`)
/// and advances by one cell on each spanned axis — which a corner-based derivation
/// (`0.0 + 0.707 → floor 0`) would not. **Zero pixels** — sim-unit voxel coords only.
#[must_use]
pub fn faced_cell(position: &Position, facing: &Facing) -> (Cell, Level) {
    // (1) Split the shooter's (cell, level) key. Position derefs to CellLevel, whose
    //     z is the storey index; the cell is its x/y.
    let key = **position;
    let cell = Cell::new(key.x, key.y);
    // The shooter's own storey — preserved unchanged in the return (AC3): the step is
    // horizontal (z = 0), so the faced cell is on the same level. The march keeps z in
    // 0..MAX_LEVELS, so the u8 conversion always succeeds; a can't-happen out-of-range
    // storey degrades to level 0 rather than panicking.
    let level = Level::new(u8::try_from(key.z).unwrap_or(0));

    // (2)-(4) Step from the cell CENTER by the facing's UNIT forward step, then floor
    //         back to a cell. Stepping from the center (not the corner) is what makes a
    //         0.707 diagonal cross the boundary; SimPos derefs to Vec3 for the add.
    let stepped = *cell_center(cell, level) + facing.forward_step();
    let (faced, _faced_level) = pos_to_cell(SimPos::new(stepped.x, stepped.y, stepped.z));

    // (5) The faced cell, on the shooter's OWN level (the forward step is horizontal).
    (faced, level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
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
}
