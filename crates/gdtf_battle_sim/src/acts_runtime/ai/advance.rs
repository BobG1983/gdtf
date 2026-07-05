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
