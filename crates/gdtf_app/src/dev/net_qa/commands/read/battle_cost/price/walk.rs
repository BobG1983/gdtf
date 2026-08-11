//! What a route costs: the sim's own pathfinder, planned through the squad's fog.

use bevy::prelude::Entity;
use gdtf_battle_sim::{
    acts::{MoveVerdict, Mover, SightWorld, can_move, move_tu_cost},
    ganger::LifeState,
    injuries::{InflictedInjuries, MovementCostFactor},
    pathfinder::{PlanningView, find_path},
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
    let Ok(path) = find_path(**row.position, to, world.grids, factor, &planning) else {
        return Quote::refused(CostRefusalNet::NoPathToCell);
    };
    let mover = Mover::new(
        actor,
        row.position,
        row.tu,
        row.stance,
        row.facing,
        row.pinned,
    );
    let sight = SightWorld::new(world.grids.occupancy, world.surface, world.tuning, |dead| {
        rows.targets
            .get(dead)
            .is_ok_and(|(_, _, life)| *life == LifeState::Dead)
    });
    Quote::quoted(
        move_tu_cost(&path),
        refused_by(can_move(mover, &to, &path, world.cover, &sight)),
    )
}

// The wire refusal one walk verdict answers with, or nothing when the walk is allowed.
const fn refused_by(verdict: MoveVerdict) -> Option<CostRefusalNet> {
    match verdict {
        MoveVerdict::Allowed => None,
        MoveVerdict::Suppressed => Some(CostRefusalNet::Suppressed),
        MoveVerdict::Unaffordable => Some(CostRefusalNet::CannotAfford),
    }
}
