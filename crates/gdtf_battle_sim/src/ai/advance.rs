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
