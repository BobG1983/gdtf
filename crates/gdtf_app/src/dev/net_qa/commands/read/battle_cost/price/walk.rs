//! What a route costs: the sim's own pathfinder, planned through the squad's fog.

use bevy::prelude::Entity;
use gdtf_battle_sim::{
    acts::{
        MoveVerdict, Mover, SightWorld, can_move, dismount_surcharge, move_tu_cost, seat_departure,
    },
    emplacement::Mounted,
    ganger::LifeState,
    injuries::{InflictedInjuries, MovementCostFactor},
    pathfinder::{Departure, PlanningView, find_path_leaving},
    visibility::FactionRelation,
};

use super::quote::Quote;
use crate::dev::net_qa::{
    commands::read::battle_cost::reads::{ActorRowItem, CostRows, LoadedWorld},
    wire::{cell::CellLevelNet, cost::CostRefusalNet},
};

/// Price the walk from where the actor stands to `dest`.
pub(super) fn move_quote(
    world: &LoadedWorld,
    rows: &CostRows,
    actor: Entity,
    row: &ActorRowItem<'_, '_>,
    dest: CellLevelNet,
) -> Quote {
    let mover_faction = *row.faction;
    let planning = PlanningView::new(world.squad, |occupant| match rows.factions.get(occupant) {
        Ok(faction) if *faction == mover_faction => FactionRelation::OwnSquad,
        _ => FactionRelation::Other,
    });
    let factor = row.injuries.map_or(
        MovementCostFactor::IDENTITY,
        InflictedInjuries::movement_cost_factor,
    );
    let to = dest.to_sim();
    let departure = departure_of(rows, row);
    let Ok(path) = find_path_leaving(&departure, to, world.grids, factor, &planning) else {
        return Quote::refused(CostRefusalNet::NoPathToCell);
    };
    let surcharge = dismount_surcharge(row.mounted, world.tuning);
    let mover = Mover::new(
        actor,
        row.position,
        row.tu,
        row.stance,
        row.facing,
        row.pinned,
        surcharge,
    );
    let sight = SightWorld::new(world.grids.occupancy, world.surface, world.tuning, |dead| {
        rows.targets
            .get(dead)
            .is_ok_and(|(_, _, life)| *life == LifeState::Dead)
    });
    Quote::quoted(
        move_tu_cost(&path, surcharge),
        refused_by(can_move(mover, &to, &path, world.cover, &sight)),
    )
}

// Where this actor's route may leave: the seat's entry cells while it is riding one.
fn departure_of(rows: &CostRows, row: &ActorRowItem<'_, '_>) -> Departure {
    let from = **row.position;
    let Some(seat) = row.mounted.and_then(Mounted::emplacement) else {
        return Departure::anywhere(from);
    };
    let Ok((_state, cell, _occupant, sides, facing)) = rows.emplacements.get(seat) else {
        return Departure::anywhere(from);
    };
    seat_departure(from, **cell, sides, facing)
}

// The wire refusal one walk verdict answers with, or nothing when the walk is allowed.
const fn refused_by(verdict: MoveVerdict) -> Option<CostRefusalNet> {
    match verdict {
        MoveVerdict::Allowed => None,
        MoveVerdict::Suppressed => Some(CostRefusalNet::Suppressed),
        MoveVerdict::Unaffordable => Some(CostRefusalNet::CannotAfford),
    }
}
