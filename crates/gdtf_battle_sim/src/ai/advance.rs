//! The ADVANCE reposition plan — step an enemy toward its goal over the omniscient
//! move fog (GTW-70 §D.2), sharing the executor's exact planning inputs.

use bevy::prelude::Entity;

use super::{
    decide::{AiTarget, plan_advance},
    snapshot::{GangerRow, row_cell_level},
};
use crate::{
    metric::CellLevel,
    occupancy::OccupancyGrid,
    pathfinder::{PlanningView, reachable_within},
    terrain::floor::FloorCostGrid,
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
};

/// (2) ADVANCE — no engageable target: step toward the nearest opposing ganger's
///     actual cell over the `OmniscientFog` move fog (the AI knows where to walk, but
///     still cannot SHOOT until `can_see` passes — §D.1/§D.2). For any non-player mover
///     `move_fog` provably selects the `OmniscientFog`, so the brain reads the IDENTICAL
///     resource `dispatch_move`'s `move_fog` selects → planner and executor share ONE fog
///     (the reposition emit-⇒-accept guarantee), without the brain needing the player
///     fog it would never select. Absent the fog (no live battle) the brain can't
///     plan and HOLDs.
///
/// Extracted verbatim from [`enemy_ai_turn`](super::brain::enemy_ai_turn)'s per-enemy
/// loop so that system stays under the line cap; returns the planned destination, or
/// `None` when no reachable step improves on the enemy's cell (the brain then HOLDs).
///
/// # Ruling: an advance CAN retrace a cell, and this is where it comes from (GTW-889)
///
/// GTW-889 reported an enemy stepping back onto a cell it had just left before moving
/// forward again. That is NOT a draw-order artifact: the sprite moves off
/// `Changed<DrawnPosition>`, the presenter writes that mirror only from act-log entries in
/// log order (its gap recovery jumps forward to the head and never rewinds), and
/// [`advance_walk`](crate::acts::movement::advance_walk) pops an accepted route's cells in
/// order, so
/// a committed walk cannot reverse mid-route. The drawn sequence IS the sim's step
/// sequence.
///
/// It comes from THIS function having no memory. Every act re-plans from the enemy's
/// current cell over a freshly computed reachable set, and the goal handed in is
/// [`pick_nearest`](super::decide::pick_nearest) re-evaluated from that same cell — so
/// when the nearest opposing ganger changes, the next plan can lead back the way the
/// enemy came. [`plan_advance`](super::decide::plan_advance) only guarantees the
/// DESTINATION strictly reduces the Chebyshev distance to the goal; the route
/// [`find_path`](crate::pathfinder::find_path) takes there does not have to, so a detour
/// around terrain can re-enter a just-left cell too. Both are pure functions of game
/// state, never of system run order.
///
/// So it is a planning-quality behaviour, not a timing or ordering defect, and it is
/// tracked as GTW-936 (a child of the GTW-71 tactical-AI epic): carry a committed advance
/// plan across acts rather than rebuilding one after every step.
#[expect(
    clippy::too_many_arguments,
    reason = "the plan borrows the brain's own reads (the enemy + snapshot rows, the \
              advance goal, the omniscient move fog, and the grid / link / tuning / \
              floor-cost planning inputs reachable_within takes); each is a distinct \
              borrow mirroring enemy_ai_turn's own argument-count carve-out — bundling \
              would only hide the reads"
)]
pub(super) fn plan_reposition(
    enemy: &GangerRow,
    goal: &AiTarget,
    rows: &[GangerRow],
    omniscient: &SquadVisibility,
    occupancy: &OccupancyGrid,
    links: &VerticalLinkGraph,
    tuning: &CombatTuning,
    floor_costs: &FloorCostGrid,
) -> Option<CellLevel> {
    let enemy_cell_level = row_cell_level(&enemy.position);
    let relation_of = |occupant: Entity| {
        rows.iter()
            .find(|row| row.entity == occupant)
            .map_or(FactionRelation::Other, |row| {
                if row.faction == enemy.faction {
                    FactionRelation::OwnSquad
                } else {
                    FactionRelation::Other
                }
            })
    };
    let planning = PlanningView::new(omniscient, relation_of);
    let reachable = reachable_within(
        enemy_cell_level,
        enemy.tu,
        occupancy,
        links,
        tuning,
        floor_costs,
        enemy.factor,
        &planning,
    );
    plan_advance(enemy_cell_level, goal.cell, &reachable)
}
