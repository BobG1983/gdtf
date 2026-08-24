//! Reachable-cells overlay for the selected shooter (debug).

use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::*,
};
use gdtf_battle_presenter::ReachableCells;
use gdtf_battle_sim::{
    acts::{DismountSurcharge, dismount_surcharge, seat_departure},
    emplacement::{EmplacementEntrySides, EmplacementFacing, Mounted},
    entity::TerrainCell,
    floor::FloorCostGrid,
    injuries::{InflictedInjuries, MovementCostFactor},
    pathfinder::{Departure, MoveGrids, PlanningView, reachable_within},
    prelude::{CellLevel, Faction, OccupancyGrid, Position, Tu},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
};

use crate::selection::resources::SelectedShooter;

/// What the overlay reads off the ganger whose reachable set it is drawing.
#[derive(QueryData)]
pub struct ReachableMoverRow {
    position: &'static Position,
    budget:   &'static Tu,
    faction:  &'static Faction,
    injuries: Option<&'static InflictedInjuries>,
    mounted:  Option<&'static Mounted>,
}

/// What one overlay pass is planned from, once the selection resolves.
struct ReachablePlan {
    departure: Departure,
    budget:    Tu,
    faction:   Faction,
    factor:    MovementCostFactor,
    surcharge: DismountSurcharge,
}

/// Recompute cells reachable within the selected shooter's TU budget.
pub fn populate_reachable_overlay(
    selected: Res<SelectedShooter>,
    actors: Query<ReachableMoverRow>,
    factions: Query<&'static Faction>,
    grids: ReachableGrids,
    mut overlay: ResMut<ReachableCells>,
) {
    let next = match resolve_selected(*selected, &actors, &grids) {
        Some(plan) => reachable_for(&plan, &factions, &grids),
        None => ReachableCells::cleared(),
    };
    if *overlay != next {
        *overlay = next;
    }
}

fn resolve_selected(
    selected: SelectedShooter,
    actors: &Query<ReachableMoverRow>,
    grids: &ReachableGrids,
) -> Option<ReachablePlan> {
    let entity = (*selected)?;
    let row = actors.get(entity).ok()?;
    let factor = row.injuries.map_or(
        MovementCostFactor::IDENTITY,
        InflictedInjuries::movement_cost_factor,
    );
    Some(ReachablePlan {
        departure: grids.departure(**row.position, row.mounted),
        budget: *row.budget,
        faction: *row.faction,
        factor,
        surcharge: dismount_surcharge(row.mounted, &grids.tuning),
    })
}

fn reachable_for(
    plan: &ReachablePlan,
    factions: &Query<&'static Faction>,
    grids: &ReachableGrids,
) -> ReachableCells {
    let mover_faction = plan.faction;
    let planning = PlanningView::new(&grids.squad, |occupant| {
        relation_to(factions, mover_faction, occupant)
    });
    let raw = reachable_within(
        &plan.departure,
        plan.budget,
        plan.surcharge,
        MoveGrids {
            occupancy:   &grids.grid,
            links:       &grids.links,
            floor_costs: &grids.floor_costs,
            tuning:      &grids.tuning,
        },
        plan.factor,
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

/// What the overlay reads off the seat a mounted ganger rides.
type ReachableSeatRow = (
    &'static TerrainCell,
    Option<&'static EmplacementEntrySides>,
    Option<&'static EmplacementFacing>,
);

/// Grid resources and seat rows needed for reachable-cell planning.
#[derive(SystemParam)]
pub struct ReachableGrids<'w, 's> {
    pub(crate) grid:        Res<'w, OccupancyGrid>,
    pub(crate) links:       Res<'w, VerticalLinkGraph>,
    pub(crate) squad:       Res<'w, SquadVisibility>,
    pub(crate) tuning:      Res<'w, CombatTuning>,
    pub(crate) floor_costs: Res<'w, FloorCostGrid>,
    pub(crate) seats:       Query<'w, 's, ReachableSeatRow>,
}

impl ReachableGrids<'_, '_> {
    /// Where a route off `from` may leave: the seat's entry cells while the mover rides one.
    fn departure(&self, from: CellLevel, mounted: Option<&Mounted>) -> Departure {
        let Some(seat) = mounted.and_then(Mounted::emplacement) else {
            return Departure::anywhere(from);
        };
        let Ok((cell, sides, facing)) = self.seats.get(seat) else {
            return Departure::anywhere(from);
        };
        seat_departure(from, **cell, sides, facing)
    }
}
