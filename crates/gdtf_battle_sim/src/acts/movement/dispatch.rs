//! Pathfind move requests and start [`WalkInProgress`].

use bevy::prelude::{MessageReader, MessageWriter, Query};

use super::{
    cost::{MoveVerdict, Mover, can_move, move_step_tu_costs},
    mount::dismount_surcharge,
    params::{MoveCommit, MovePlanningView, MoverRow, PathfindingGrids, SuppressionGate},
    signals::{MoveRejected, MoveRejection},
};
use crate::{
    acts::{movement::WalkInProgress, request::MoveRequested},
    injuries::{InflictedInjuries, MovementCostFactor},
    metric::CellLevel,
    pathfinder::find_path_leaving,
};

/// Pathfind and either reject or insert a walk component.
/// A mounted mover leaves by a seat entry cell and pays the exit act on top of the route.
pub fn dispatch_move(
    mut requests: MessageReader<MoveRequested>,
    actors: Query<MoverRow>,
    grids: PathfindingGrids,
    view: MovePlanningView,
    gate: SuppressionGate,
    mut commit: MoveCommit,
    mut rejects: MessageWriter<MoveRejected>,
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
        let departure = commit.departure(start, actor.mounted);

        let Ok(path) =
            find_path_leaving(&departure, request.dest, grids.grids(), factor, &planning)
        else {
            rejects.write(MoveRejected::new(request.actor, MoveRejection::Unreachable));
            continue;
        };

        let surcharge = dismount_surcharge(actor.mounted, grids.tuning());
        let mover = Mover::new(
            request.actor,
            actor.position,
            actor.tu,
            actor.stance,
            actor.facing,
            gate.on(request.actor),
            surcharge,
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
            commit.start_walk(
                request.actor,
                WalkInProgress::new(&cells[1..], &move_step_tu_costs(&path, surcharge)),
            );
        }
    }
}
