//! Relocated unit tests for the movement verb (GTW-201 wave 22 — moved verbatim from
//! the former inline `#[cfg(test)] mod tests`).

use bevy::prelude::{Entity, World};

use crate::{
    ganger::{LifeState, Position, Tu},
    metric::{Cell, CellLevel, Level},
    move_acts::{MoveOutcome, move_ganger},
    occupancy::{OccupancyGrid, TerrainKind},
    tuning::MoveCosts,
};

/// Mint a valid [`Entity`] handle for a test fixture (no `from_raw` in 0.18) — spawn
/// an empty entity in a fresh world and take its id.
fn an_entity() -> Entity {
    World::new().spawn_empty().id()
}

/// A `(cell, level)` at `(x, y, 0)` — the storey-0 plane the tests move within.
fn at(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

// === AC3 — a VALID move sets Position == dest AND drops Tu by EXACTLY the
// destination terrain's move cost; covered for BOTH an Open dest and a
// destroyed-Cover dest (proving the cost is terrain-determined, not flat). ===

#[test]
fn valid_move_onto_open_dest_steps_and_spends_the_open_terrain_cost() {
    let grid = OccupancyGrid::new(); // every slot defaults to TerrainKind::Open, empty
    let costs = MoveCosts::default();
    let source = at(10, 10);
    let dest = at(11, 10);
    let mut position = Position::new(source);
    let mut tu = Tu::new(100);
    let tu_before = *tu;

    let outcome = move_ganger(
        &mut position,
        &mut tu,
        LifeState::Alive,
        dest,
        &grid,
        &costs,
    );

    assert_eq!(
        outcome,
        MoveOutcome::Moved,
        "a valid Open-dest move succeeds"
    );
    assert_eq!(
        position,
        Position::new(dest),
        "a successful move sets Position to the destination",
    );
    // The drop equals exactly the LOOKED-UP Open-terrain cost — a relation to the
    // tuning value, never a pinned magnitude.
    assert_eq!(
        tu_before - *tu,
        *costs.cost(grid.terrain(&dest)),
        "the Tu drop must equal the destination Open terrain's looked-up move cost",
    );
    // And the dest really did read Open (the cost we asserted against).
    assert_eq!(
        grid.terrain(&dest),
        TerrainKind::Open,
        "precondition: the Open dest reads TerrainKind::Open",
    );
}

#[test]
fn valid_move_onto_destroyed_cover_dest_steps_and_spends_the_cover_terrain_cost() {
    let mut grid = OccupancyGrid::new();
    let dest = at(11, 10);
    // A Cover cell that has been DESTROYED: is_blocked is false (so it is a legal
    // dest) but grid.terrain still reads TerrainKind::Cover — so the COVER cost
    // applies, proving the cost is terrain-determined, not flat.
    grid.set_terrain(dest, TerrainKind::Cover);
    grid.mark_cover_destroyed(dest);
    let costs = MoveCosts::default();
    let mut position = Position::new(at(10, 10));
    let mut tu = Tu::new(100);
    let tu_before = *tu;

    // Sanity: the dest is unblocked (destroyed cover) but still reads Cover terrain.
    assert!(
        !grid.is_blocked(&dest),
        "precondition: a destroyed-cover dest is unblocked",
    );
    assert_eq!(
        grid.terrain(&dest),
        TerrainKind::Cover,
        "precondition: a destroyed-cover dest still reads TerrainKind::Cover",
    );

    let outcome = move_ganger(
        &mut position,
        &mut tu,
        LifeState::Alive,
        dest,
        &grid,
        &costs,
    );

    assert_eq!(
        outcome,
        MoveOutcome::Moved,
        "a valid destroyed-Cover-dest move succeeds",
    );
    assert_eq!(position, Position::new(dest), "the move steps to the dest");
    // The drop equals exactly the LOOKED-UP Cover-terrain cost (distinct from Open).
    assert_eq!(
        tu_before - *tu,
        *costs.cost(grid.terrain(&dest)),
        "the Tu drop must equal the destination Cover terrain's looked-up move cost",
    );
}

// === AC4 — ALL-GATES composite: an AFFORDABLE move onto an OCCUPIED dest is a total
// no-op (occupancy beats affordability). ===

#[test]
fn affordable_move_onto_occupied_dest_is_a_total_no_op() {
    let mut grid = OccupancyGrid::new();
    let dest = at(11, 10);
    // Another occupant already stands at the dest.
    grid.set_occupant(dest, Some(an_entity()));
    let costs = MoveCosts::default();
    let source = at(10, 10);
    let mut position = Position::new(source);
    let mut tu = Tu::new(200); // ample TU — affordability is NOT the failing gate
    let pos_before = position;
    let tu_before = tu;

    let outcome = move_ganger(
        &mut position,
        &mut tu,
        LifeState::Alive,
        dest,
        &grid,
        &costs,
    );

    assert_eq!(
        outcome,
        MoveOutcome::Blocked,
        "an occupied dest blocks even an affordable move",
    );
    assert_eq!(
        position, pos_before,
        "occupancy no-op leaves Position unchanged"
    );
    assert_eq!(tu, tu_before, "occupancy no-op leaves Tu unchanged");
}

// === AC5 — EACH failing gate is a total no-op (Position AND Tu unchanged), swept
// independently: out-of-bounds, blocked (Wall / Cover), occupied, Downed, Dead,
// unaffordable. A relation per gate, no pinned numbers. ===

/// Run a move that SHOULD fail and assert it is a total no-op (Position AND Tu
/// unchanged) and returns [`MoveOutcome::Blocked`]. `label` names the failing gate.
fn assert_blocked_no_op(
    grid: &OccupancyGrid,
    life: LifeState,
    dest: CellLevel,
    tu_start: u8,
    label: &str,
) {
    let costs = MoveCosts::default();
    let source = at(10, 10);
    let mut position = Position::new(source);
    let mut tu = Tu::new(tu_start);
    let pos_before = position;
    let tu_before = tu;

    let outcome = move_ganger(&mut position, &mut tu, life, dest, grid, &costs);

    assert_eq!(
        outcome,
        MoveOutcome::Blocked,
        "{label}: move must be blocked"
    );
    assert_eq!(position, pos_before, "{label}: Position must be unchanged");
    assert_eq!(tu, tu_before, "{label}: Tu must be unchanged");
}

#[test]
fn out_of_bounds_dest_is_a_total_no_op() {
    let grid = OccupancyGrid::new();
    // A cell well outside the 60×60×8 grid — `slot` reads None (out of bounds).
    let dest = at(1000, 1000);
    assert!(
        grid.slot(&dest).is_none(),
        "precondition: the dest is out of bounds",
    );
    assert_blocked_no_op(&grid, LifeState::Alive, dest, 200, "out-of-bounds dest");
}

#[test]
fn blocked_wall_dest_is_a_total_no_op() {
    let mut grid = OccupancyGrid::new();
    let dest = at(11, 10);
    grid.set_terrain(dest, TerrainKind::Wall);
    assert!(grid.is_blocked(&dest), "precondition: the Wall dest blocks");
    assert_blocked_no_op(&grid, LifeState::Alive, dest, 200, "blocked Wall dest");
}

#[test]
fn blocked_standing_cover_dest_is_a_total_no_op() {
    let mut grid = OccupancyGrid::new();
    let dest = at(11, 10);
    // Standing (NOT destroyed) cover blocks.
    grid.set_terrain(dest, TerrainKind::Cover);
    assert!(
        grid.is_blocked(&dest),
        "precondition: a standing Cover dest blocks",
    );
    assert_blocked_no_op(&grid, LifeState::Alive, dest, 200, "blocked Cover dest");
}

#[test]
fn occupied_dest_is_a_total_no_op() {
    let mut grid = OccupancyGrid::new();
    let dest = at(11, 10);
    grid.set_occupant(dest, Some(an_entity()));
    assert_blocked_no_op(&grid, LifeState::Alive, dest, 200, "occupied dest");
}

#[test]
fn downed_actor_cannot_move() {
    let grid = OccupancyGrid::new();
    let dest = at(11, 10); // an otherwise-valid Open, empty, in-bounds dest
    assert_blocked_no_op(&grid, LifeState::Downed, dest, 200, "Downed actor");
}

#[test]
fn dead_actor_cannot_move() {
    let grid = OccupancyGrid::new();
    let dest = at(11, 10);
    assert_blocked_no_op(&grid, LifeState::Dead, dest, 200, "Dead actor");
}

#[test]
fn unaffordable_move_is_a_total_no_op() {
    let grid = OccupancyGrid::new(); // Open, empty dest
    let costs = MoveCosts::default();
    let dest = at(11, 10);
    // One TU below the dest terrain's looked-up cost — the affordability gate fails.
    let cost = *costs.cost(grid.terrain(&dest));
    assert!(
        cost > 0,
        "precondition: the move cost is positive so 'short by one' is meaningful"
    );
    let short = cost - 1;
    assert_blocked_no_op(&grid, LifeState::Alive, dest, short, "unaffordable move");
}
