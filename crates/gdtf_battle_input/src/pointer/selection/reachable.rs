//! Reachable-cells overlay for the selected shooter (debug).

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::ReachableCells;
use gdtf_battle_sim::{
    floor::FloorCostGrid,
    injuries::{InflictedInjuries, MovementCostFactor},
    pathfinder::{PlanningView, reachable_within},
    prelude::{CellLevel, Faction, OccupancyGrid, Position, Tu},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
};

use crate::selection::resources::SelectedShooter;

/// Recompute cells reachable within the selected shooter's TU budget.
pub fn populate_reachable_overlay(
    selected: Res<SelectedShooter>,
    actors: Query<(&Position, &Tu, &Faction, Option<&InflictedInjuries>)>,
    factions: Query<&'static Faction>,
    grids: ReachableGrids,
    mut overlay: ResMut<ReachableCells>,
) {
    let next = match resolve_selected(*selected, &actors) {
        Some((start, budget, mover_faction, factor)) => {
            reachable_for(start, budget, mover_faction, factor, &factions, &grids)
        }
        None => ReachableCells::cleared(),
    };
    if *overlay != next {
        *overlay = next;
    }
}

fn resolve_selected(
    selected: SelectedShooter,
    actors: &Query<(&Position, &Tu, &Faction, Option<&InflictedInjuries>)>,
) -> Option<(CellLevel, Tu, Faction, MovementCostFactor)> {
    let entity = (*selected)?;
    let (position, &budget, &mover_faction, injuries) = actors.get(entity).ok()?;
    let factor = injuries.map_or(
        MovementCostFactor::IDENTITY,
        InflictedInjuries::movement_cost_factor,
    );
    Some((**position, budget, mover_faction, factor))
}

fn reachable_for(
    start: CellLevel,
    budget: Tu,
    mover_faction: Faction,
    factor: MovementCostFactor,
    factions: &Query<&'static Faction>,
    grids: &ReachableGrids,
) -> ReachableCells {
    let planning = PlanningView::new(&grids.squad, |occupant| {
        relation_to(factions, mover_faction, occupant)
    });
    let raw = reachable_within(
        start,
        budget,
        &grids.grid,
        &grids.links,
        &grids.tuning,
        &grids.floor_costs,
        factor,
        &planning,
    );
    ReachableCells::new(raw)
}

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

/// Grid resources needed for reachable-cell planning.
#[derive(SystemParam)]
pub struct ReachableGrids<'w> {
    pub(crate) grid:        Res<'w, OccupancyGrid>,
    pub(crate) links:       Res<'w, VerticalLinkGraph>,
    pub(crate) squad:       Res<'w, SquadVisibility>,
    pub(crate) tuning:      Res<'w, CombatTuning>,
    pub(crate) floor_costs: Res<'w, FloorCostGrid>,
}
