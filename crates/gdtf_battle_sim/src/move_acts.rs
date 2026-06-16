//! The **movement verb** — the FIRST movement act in the authoritative sim (GTW-234).
//!
//! [`move_ganger`] is the pure, render-free verb that steps a ganger one cell to a
//! destination `(cell, level)`, charging a **terrain-determined** TU cost. It mirrors the
//! [`crate::posture`] / [`crate::downed_acts`] verb-module precedent: it takes the landed
//! ganger newtypes by reference plus the read-only [`OccupancyGrid`] and the
//! [`MoveCosts`] tuning table, with **no [`World`](bevy::ecs::world::World) access**, so
//! it unit-tests against bare component values with no ECS plumbing. The
//! message-driven seam ([`crate::acts::MoveRequested`] + `dispatch_move`) lives in
//! [`crate::acts`]; this module owns the rule.
//!
//! ## Terrain-determined cost (the core)
//!
//! The move's TU cost is NOT a flat per-cell constant — it is the DESTINATION cell's
//! terrain movement cost (`docs/combat/combat.md` L34: "step" costs TUs; the user ruling
//! "the floor tile you cross — the terrain determines the cost"). The verb reads
//! [`OccupancyGrid::terrain`] at the destination, looks its cost up in the
//! per-[`TerrainKind`](crate::occupancy::TerrainKind) [`MoveCosts`] table, and charges
//! exactly that. The granularity is per-`TerrainKind` (coarse — Open / Cover / Wall);
//! richer per-floor-type costs are a follow-up. The cost is applied flat regardless of
//! step direction (a diagonal-costs-more differential is deferred).
//!
//! ## The gates (ALL must pass, or it is a TOTAL no-op)
//!
//! A move succeeds only when EVERY gate holds — and the pass/fail decision is computed
//! BEFORE any mutation, so a failing move touches NEITHER [`Position`] NOR [`Tu`]:
//!
//! - the actor is [`LifeState::Alive`] (a Downed / Dead ganger cannot move);
//! - the destination is **in-bounds** (a real in-grid `(cell, level)`,
//!   [`OccupancyGrid::slot`] returns `Some`) AND **not blocked**
//!   ([`OccupancyGrid::is_blocked`] is `false` — a standing Wall / Cover cell is
//!   rejected; a destroyed-cover cell passes, its terrain still reads
//!   [`TerrainKind::Cover`](crate::occupancy::TerrainKind::Cover) so its per-terrain cost
//!   still applies);
//! - the destination is **unoccupied** ([`OccupancyGrid::occupant`] is `None`);
//! - the move is **affordable** ([`can_spend_tu`] against the looked-up terrain cost).
//!
//! Occupancy beats affordability: an affordable move onto an occupied cell is a no-op.
//! On success the verb writes `Position = Position::new(dest)` and spends the looked-up
//! cost via [`spend_tu`] — and writes ONLY [`Position`]: the occupancy-grid slot
//! maintenance is the landed [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers)
//! reactor on `Changed<`[`Position`]`>`, never a grid write here.

use crate::{
    ganger::{LifeState, Position, Tu},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    tu::{can_spend_tu, spend_tu},
    tuning::MoveCosts,
};

/// The result of a [`move_ganger`] call — whether the ganger actually stepped.
///
/// A named outcome (no-bare-types: a move's success is a domain value, not a bare
/// `bool`), mirroring the [`crate::posture`] verbs' boolean returns made explicit. The
/// caller (`dispatch_move`) does not branch on it today — the move's effect is the
/// in-world `Position` / `Tu` mutation the presenter observes via change detection — but
/// it makes the verb's contract testable: [`Moved`](MoveOutcome::Moved) iff every gate
/// passed and the step was taken, [`Blocked`](MoveOutcome::Blocked) on any gate failure
/// (a TOTAL no-op).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveOutcome {
    /// Every gate passed: the ganger stepped to the destination and the looked-up
    /// terrain cost was spent.
    Moved,
    /// At least one gate failed: NEITHER [`Position`] NOR [`Tu`] was touched (a total
    /// no-op — the actor stayed put and spent nothing).
    Blocked,
}

/// Step `position`'s ganger one cell to `dest`, charging the DESTINATION terrain's move
/// cost — a pure, render-free verb gated by liveness, bounds, blocking, occupancy, and
/// affordability.
///
/// ALL gates must pass (see the module docs); the pass/fail decision is computed BEFORE
/// any mutation, so a failing move is a TOTAL no-op (NEITHER `position` NOR `tu` is
/// touched) and returns [`MoveOutcome::Blocked`]. On success it writes
/// `*position = Position::new(dest)` and spends the looked-up terrain cost via
/// [`spend_tu`] (saturating), returning [`MoveOutcome::Moved`].
///
/// The TU cost is **terrain-determined**: `move_costs.cost(grid.terrain(&dest))` — the
/// destination cell's [`TerrainKind`](crate::occupancy::TerrainKind) movement cost, NOT a
/// flat constant (the user ruling: the floor tile crossed determines the cost). The verb
/// writes ONLY [`Position`]; the occupancy-grid slot maintenance is the landed
/// [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) `Changed<Position>`
/// reactor — there is no grid write here.
pub fn move_ganger(
    position: &mut Position,
    tu: &mut Tu,
    life: LifeState,
    dest: CellLevel,
    grid: &OccupancyGrid,
    move_costs: &MoveCosts,
) -> MoveOutcome {
    // The terrain-determined cost: look the destination cell's terrain up in the
    // per-TerrainKind table, then convert the per-cell MoveCost leaf to a Tu amount.
    let cost = Tu::new(*move_costs.cost(grid.terrain(&dest)));

    // Compute the full pass/fail decision BEFORE mutating, so any gate failure is a
    // TOTAL no-op (neither Position nor Tu touched).
    let alive = matches!(life, LifeState::Alive);
    // In-bounds: `slot` returns `None` for an out-of-grid `(cell, level)`. `is_blocked`
    // alone is insufficient — it reads Open/not-blocked for an out-of-range cell — so the
    // in-grid check is combined with the not-blocked check (a standing Wall / Cover dest
    // blocks; a destroyed-cover dest passes).
    let in_bounds = grid.slot(&dest).is_some();
    let unblocked = !grid.is_blocked(&dest);
    let unoccupied = grid.occupant(&dest).is_none();
    let affordable = can_spend_tu(tu, cost);

    if !(alive && in_bounds && unblocked && unoccupied && affordable) {
        return MoveOutcome::Blocked;
    }

    // Every gate passed — take the step and charge the looked-up terrain cost. Writes
    // ONLY Position (the grid slot is maintained by the Changed<Position> reactor).
    *position = Position::new(dest);
    spend_tu(tu, cost);
    MoveOutcome::Moved
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{Entity, World};

    use super::*;
    use crate::{
        metric::{Cell, CellLevel, Level},
        occupancy::TerrainKind,
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
}
