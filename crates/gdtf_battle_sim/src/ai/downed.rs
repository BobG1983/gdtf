//! Plan an execute on an adjacent downed foe, or a stabilize on an adjacent bleeding ally.

use super::{
    decide::{AiTarget, pick_nearest},
    params::{AiActRequests, AiPlanningGrids},
    snapshot::GangerRow,
};
use crate::acts::{
    ExecuteDownedRequested, StabilizeDownedRequested,
    downed::{Actor, DownedTarget, can_execute, can_stabilize},
};

/// The downed act one enemy takes this frame.
pub(super) enum DownedAct {
    /// Finish an adjacent downed foe.
    Execute(ExecuteDownedRequested),
    /// Stop an adjacent downed ally bleeding.
    Stabilize(StabilizeDownedRequested),
}

/// Write the planned act on the writer that carries it.
pub(super) fn write_downed_act(orders: &mut AiActRequests, act: DownedAct) {
    match act {
        DownedAct::Execute(execute) => {
            orders.execute.write(execute);
        }
        DownedAct::Stabilize(stabilize) => {
            orders.stabilize.write(stabilize);
        }
    }
}

// The acting ganger as both downed gates read it.
const fn actor_of(enemy: &GangerRow) -> Actor {
    Actor {
        pos:     enemy.position,
        life:    enemy.life,
        faction: enemy.faction,
    }
}

// One snapshot row as both downed gates read it.
const fn target_of(row: &GangerRow) -> DownedTarget {
    DownedTarget {
        pos:          row.position,
        life:         row.life,
        faction:      row.faction,
        bleeding_out: row.bleeding_out,
    }
}

// Nearest row the gate accepts, so several passing rows resolve the same way every replay.
fn nearest_passing(
    enemy: &GangerRow,
    rows: &[GangerRow],
    gate: impl Fn(&DownedTarget) -> bool,
) -> Option<AiTarget> {
    let passing: Vec<AiTarget> = rows
        .iter()
        .filter(|row| gate(&target_of(row)))
        .map(|row| AiTarget::new(row.entity, row.position.cell(), row.position.level()))
        .collect();
    pick_nearest(enemy.position.cell(), enemy.position.level(), &passing)
}

/// Execute an adjacent downed foe, else stabilize an adjacent bleeding ally.
/// `can_execute` is scanned first, and both gates fold the TU check.
pub(super) fn plan_downed_act(
    enemy: &GangerRow,
    rows: &[GangerRow],
    grids: &AiPlanningGrids,
) -> Option<DownedAct> {
    let actor = actor_of(enemy);
    let tuning = grids.tuning();
    if let Some(foe) = nearest_passing(enemy, rows, |target| {
        *can_execute(&actor, target, &enemy.tu, tuning)
    }) {
        return Some(DownedAct::Execute(ExecuteDownedRequested::new(
            enemy.entity,
            foe.entity,
        )));
    }
    let ally = nearest_passing(enemy, rows, |target| {
        *can_stabilize(&actor, target, &enemy.tu, tuning)
    })?;
    Some(DownedAct::Stabilize(StabilizeDownedRequested::new(
        enemy.entity,
        ally.entity,
    )))
}
