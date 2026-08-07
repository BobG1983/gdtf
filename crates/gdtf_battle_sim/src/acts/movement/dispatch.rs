//! Pathfind move requests and start [`WalkInProgress`].

use bevy::prelude::{Commands, MessageReader, MessageWriter, Query};

use super::{
    cost::{can_move, move_step_tu_costs},
    params::{MovePlanningView, PathfindingGrids, SuppressionGate},
    signals::{MoveRejected, MoveRejection},
};
use crate::{
    acts::{movement::WalkInProgress, request::MoveRequested},
    ganger::{Faction, Position, Tu},
    injuries::{InflictedInjuries, MovementCostFactor},
    metric::CellLevel,
    pathfinder::find_path,
};

/// Pathfind and either reject or insert a walk component.
pub fn dispatch_move(
    mut requests: MessageReader<MoveRequested>,
    actors: Query<(
        &'static Position,
        &'static Tu,
        &'static Faction,
        Option<&'static InflictedInjuries>,
    )>,
    grids: PathfindingGrids,
    view: MovePlanningView,
    gate: SuppressionGate,
    mut rejects: MessageWriter<MoveRejected>,
    mut commands: Commands,
) {
    for request in requests.read() {
        let Ok((position, tu, &mover_faction, injuries)) = actors.get(request.actor) else {
            continue;
        };

        let factor = injuries.map_or(
            MovementCostFactor::IDENTITY,
            InflictedInjuries::movement_cost_factor,
        );

        let start: CellLevel = **position;
        let planning = view.of(mover_faction);

        let Ok(path) = find_path(start, request.dest, grids.grids(), factor, &planning) else {
            rejects.write(MoveRejected::new(request.actor, MoveRejection::Unreachable));
            continue;
        };

        if !*gate.allows(request.actor, &start, &request.dest) {
            rejects.write(MoveRejected::new(request.actor, MoveRejection::Suppressed));
            continue;
        }

        if !*can_move(tu, &path) {
            rejects.write(MoveRejected::new(
                request.actor,
                MoveRejection::Unaffordable,
            ));
            continue;
        }

        let cells = path.cells();
        if cells.len() > 1 {
            commands
                .entity(request.actor)
                .insert(WalkInProgress::new(&cells[1..], move_step_tu_costs(&path)));
        }
    }
}
