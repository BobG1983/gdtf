//! Plan a deliberate shove, but only one that puts the target over an edge.

use super::{
    decide::{AiTarget, pick_nearest},
    params::AiPlanningGrids,
    snapshot::GangerRow,
};
use crate::{
    acts::{
        ShoveActor, ShoveOutcome, ShoveRequested, ShoveTarget, can_shove, resolve_shove,
        shove_tu_cost,
    },
    tu::can_spend_tu,
};

/// The nearest adjacent foe this push would drop off an edge, as a shove request.
/// Only a push `resolve_shove` answers `Fell` is worth the TU.
pub(super) fn plan_shove(
    enemy: &GangerRow,
    targets: &[GangerRow],
    grids: &AiPlanningGrids,
) -> Option<ShoveRequested> {
    // `can_shove` is TU-blind, so the pool is checked here the way `dispatch_shove` does.
    if !*can_spend_tu(&enemy.tu, shove_tu_cost(grids.tuning())) {
        return None;
    }
    let shover = ShoveActor {
        position: enemy.position,
        faction:  enemy.faction,
    };
    let march = grids.march();
    let mut over_the_edge: Vec<AiTarget> = Vec::new();
    for row in targets {
        let target = ShoveTarget {
            position: row.position,
            faction:  row.faction,
            life:     row.life,
        };
        if !*can_shove(&shover, &target) {
            continue;
        }
        let outcome = resolve_shove(
            enemy.position,
            row.position,
            row.entity,
            march.surface,
            march.occupancy,
        );
        if !matches!(outcome, ShoveOutcome::Fell { .. }) {
            continue;
        }
        over_the_edge.push(AiTarget::new(
            row.entity,
            row.position.cell(),
            row.position.level(),
        ));
    }
    let picked = pick_nearest(
        enemy.position.cell(),
        enemy.position.level(),
        &over_the_edge,
    )?;
    Some(ShoveRequested::new(enemy.entity, picked.entity))
}
