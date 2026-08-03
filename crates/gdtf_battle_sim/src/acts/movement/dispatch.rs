//! Pathfind move requests and start [`WalkInProgress`].

use bevy::prelude::{Commands, Entity, MessageReader, MessageWriter, Query, Res};

use super::{
    signals::{MoveRejected, MoveRejection},
    suppression_gate::suppressed_move_legal,
};
use crate::{
    acts::{movement::WalkInProgress, request::MoveRequested},
    battle::PlayerFaction,
    cover::CoverLedger,
    ganger::{Faction, Position, Suppressed, Tu},
    injuries::{InflictedInjuries, MovementCostFactor},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    pathfinder::{PlanningView, find_path},
    terrain::floor::FloorCostGrid,
    tu::can_spend_tu,
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, OmniscientFog, SquadVisibility, move_fog},
};

fn relation_to(
    factions: &Query<&'static Faction>,
    mover_faction: Faction,
    occupant: Entity,
) -> FactionRelation {
    match factions.get(occupant) {
        Ok(faction) if *faction == mover_faction => FactionRelation::OwnSquad,
        _ => FactionRelation::Other,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "actor, faction, suppression, and route-gate resources are separate reads"
)]
/// Pathfind and either reject or insert a walk component.
pub fn dispatch_move(
    mut requests: MessageReader<MoveRequested>,
    actors: Query<(
        &'static Position,
        &'static Tu,
        &'static Faction,
        Option<&'static InflictedInjuries>,
    )>,
    factions: Query<&'static Faction>,
    suppressed: Query<&'static Suppressed>,
    grid: Res<OccupancyGrid>,
    links: Res<VerticalLinkGraph>,
    squad: Res<SquadVisibility>,
    tuning: Res<CombatTuning>,
    floor_costs: Res<FloorCostGrid>,
    cover: Res<CoverLedger>,
    player: Option<Res<PlayerFaction>>,
    omniscient: Option<Res<OmniscientFog>>,
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

        let player_fog: &SquadVisibility = &squad;
        let planning_fog: &SquadVisibility = match (player.as_deref(), omniscient.as_deref()) {
            (Some(player), Some(omniscient)) => {
                move_fog(mover_faction, **player, player_fog, omniscient)
            }
            _ => player_fog,
        };

        let planning = PlanningView::new(planning_fog, |occupant| {
            relation_to(&factions, mover_faction, occupant)
        });

        let Ok(path) = find_path(
            start,
            request.dest,
            &grid,
            &links,
            &tuning,
            &floor_costs,
            factor,
            &planning,
        ) else {
            rejects.write(MoveRejected::new(request.actor, MoveRejection::Unreachable));
            continue;
        };

        if let Ok(suppressed) = suppressed.get(request.actor)
            && !*suppressed_move_legal(&start, &request.dest, &suppressed.from, &cover)
        {
            rejects.write(MoveRejected::new(request.actor, MoveRejection::Suppressed));
            continue;
        }

        if !*can_spend_tu(tu, path.total()) {
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
                .insert(WalkInProgress::new(&cells[1..], path.steps()));
        }
    }
}
