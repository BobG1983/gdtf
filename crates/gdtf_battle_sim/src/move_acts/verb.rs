//! The movement verb implementation — [`MoveOutcome`] + [`move_ganger`]. See the module
//! docs (`super`) for the terrain-cost model and the gate set.

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
