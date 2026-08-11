//! Pathfind move requests and start [`WalkInProgress`].

use bevy::prelude::{Commands, MessageReader, MessageWriter, Query};

use super::{
    cost::{MoveVerdict, Mover, can_move, move_step_tu_costs},
    params::{MovePlanningView, MoverRow, PathfindingGrids, SuppressionGate},
    signals::{MoveRejected, MoveRejection},
};
use crate::{
    acts::{movement::WalkInProgress, request::MoveRequested},
    injuries::{InflictedInjuries, MovementCostFactor},
    metric::CellLevel,
    pathfinder::find_path,
};

/// Pathfind and either reject or insert a walk component.
pub fn dispatch_move(
    mut requests: MessageReader<MoveRequested>,
    actors: Query<MoverRow>,
    grids: PathfindingGrids,
    view: MovePlanningView,
    gate: SuppressionGate,
    mut rejects: MessageWriter<MoveRejected>,
    mut commands: Commands,
) {
    for request in requests.read() {
        let Ok(actor) = actors.get(request.actor) else {
            continue;
        };

        let factor = actor.injuries.map_or(
            MovementCostFactor::IDENTITY,
            InflictedInjuries::movement_cost_factor,
        );

        let start: CellLevel = **actor.position;
        let planning = view.of(*actor.faction);

        let Ok(path) = find_path(start, request.dest, grids.grids(), factor, &planning) else {
            rejects.write(MoveRejected::new(request.actor, MoveRejection::Unreachable));
            continue;
        };

        let mover = Mover::new(
            request.actor,
            actor.position,
            actor.tu,
            actor.stance,
            actor.facing,
            gate.on(request.actor),
        );
        match can_move(mover, &request.dest, &path, gate.cover(), &gate.sight()) {
            MoveVerdict::Suppressed => {
                rejects.write(MoveRejected::new(request.actor, MoveRejection::Suppressed));
                continue;
            }
            MoveVerdict::Unaffordable => {
                rejects.write(MoveRejected::new(
                    request.actor,
                    MoveRejection::Unaffordable,
                ));
                continue;
            }
            MoveVerdict::Allowed => {}
        }

        let cells = path.cells();
        if cells.len() > 1 {
            commands
                .entity(request.actor)
                .insert(WalkInProgress::new(&cells[1..], move_step_tu_costs(&path)));
        }
    }
}
