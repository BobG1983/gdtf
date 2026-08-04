//! Plan a move that gets closer to a goal target.

use bevy::prelude::Entity;

use super::{
    decide::{AiTarget, plan_advance},
    snapshot::{GangerRow, row_cell_level},
};
use crate::{
    metric::CellLevel,
    pathfinder::{MoveGrids, PlanningView, reachable_within},
    visibility::{FactionRelation, SquadVisibility},
};

/// Best reachable cell that advances toward the goal, or None if none improves distance.
pub(super) fn plan_reposition(
    enemy: &GangerRow,
    goal: &AiTarget,
    rows: &[GangerRow],
    omniscient: &SquadVisibility,
    terrain: MoveGrids<'_>,
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
    let reachable = reachable_within(enemy_cell_level, enemy.tu, terrain, enemy.factor, &planning);
    plan_advance(enemy_cell_level, goal.cell, &reachable)
}
