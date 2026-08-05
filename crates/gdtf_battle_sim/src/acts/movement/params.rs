//! System params for move dispatch: pathfinding grids, planning fog, suppression.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, Query, Res},
};

use super::suppression_gate::{SuppressedMoveLegal, suppressed_move_legal};
use crate::{
    battle::PlayerFaction,
    cover::CoverLedger,
    ganger::{Faction, Suppressed},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    pathfinder::{MoveGrids, PlanningView},
    terrain::floor::FloorCostGrid,
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, OmniscientFog, SquadVisibility, move_fog},
};

/// Occupancy, vertical links, floor costs, and tuning a route is planned against.
#[derive(SystemParam)]
pub struct PathfindingGrids<'w> {
    occupancy:   Res<'w, OccupancyGrid>,
    links:       Res<'w, VerticalLinkGraph>,
    floor_costs: Res<'w, FloorCostGrid>,
    tuning:      Res<'w, CombatTuning>,
}

impl PathfindingGrids<'_> {
    pub(super) fn grids(&self) -> MoveGrids<'_> {
        MoveGrids {
            occupancy:   &self.occupancy,
            links:       &self.links,
            floor_costs: &self.floor_costs,
            tuning:      &self.tuning,
        }
    }
}

/// The fog and faction lookup one mover plans a route through.
#[derive(SystemParam)]
pub struct MovePlanningView<'w, 's> {
    squad:      Res<'w, SquadVisibility>,
    player:     Option<Res<'w, PlayerFaction>>,
    omniscient: Option<Res<'w, OmniscientFog>>,
    factions:   Query<'w, 's, &'static Faction>,
}

impl MovePlanningView<'_, '_> {
    /// Planning view for a mover of this faction.
    pub(super) fn of(
        &self,
        mover: Faction,
    ) -> PlanningView<'_, impl Fn(Entity) -> FactionRelation + '_> {
        let player_fog: &SquadVisibility = &self.squad;
        let fog: &SquadVisibility = match (self.player.as_deref(), self.omniscient.as_deref()) {
            (Some(player), Some(omniscient)) => move_fog(mover, **player, player_fog, omniscient),
            _ => player_fog,
        };
        PlanningView::new(fog, move |occupant| match self.factions.get(occupant) {
            Ok(faction) if *faction == mover => FactionRelation::OwnSquad,
            _ => FactionRelation::Other,
        })
    }
}

/// Whether a suppressed mover may break away, given the cover between them.
#[derive(SystemParam)]
pub struct SuppressionGate<'w, 's> {
    suppressed: Query<'w, 's, &'static Suppressed>,
    cover:      Res<'w, CoverLedger>,
}

impl SuppressionGate<'_, '_> {
    /// Legal unless the actor is suppressed and the step does not break away.
    pub(super) fn allows(
        &self,
        actor: Entity,
        start: &CellLevel,
        dest: &CellLevel,
    ) -> SuppressedMoveLegal {
        match self.suppressed.get(actor) {
            Ok(suppressed) => suppressed_move_legal(start, dest, &suppressed.from, &self.cover),
            Err(_) => SuppressedMoveLegal::new(true),
        }
    }
}
