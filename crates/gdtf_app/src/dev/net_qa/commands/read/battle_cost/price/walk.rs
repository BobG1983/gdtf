//! What a route costs: the sim's own pathfinder, planned through the squad's fog.

use gdtf_battle_sim::{
    acts::{can_move, move_tu_cost},
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
    row: &ActorRowItem<'_, '_>,
    dest: CellLevelNet,
) -> Quote {
    let mover = *row.faction;
    let planning = PlanningView::new(world.squad, |occupant| match rows.factions.get(occupant) {
        Ok(faction) if *faction == mover => FactionRelation::OwnSquad,
        _ => FactionRelation::Other,
    });
    let factor = row.injuries.map_or(
        MovementCostFactor::IDENTITY,
        InflictedInjuries::movement_cost_factor,
    );
    let Ok(path) = find_path(
        **row.position,
        dest.to_sim(),
        world.grids,
        factor,
        &planning,
    ) else {
        return Quote::refused(CostRefusalNet::NoPathToCell);
    };
    Quote::quoted(
        move_tu_cost(&path),
        (!*can_move(row.tu, &path)).then_some(CostRefusalNet::CannotAfford),
    )
}
