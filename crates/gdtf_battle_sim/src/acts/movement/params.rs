//! System params for move dispatch: pathfinding grids, planning fog, suppression.

use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::{Commands, Entity, Query, Res},
};

use super::{mount::seat_departure, sight::SightWorld, walk::WalkInProgress};
use crate::{
    battle::PlayerFaction,
    cover::CoverLedger,
    ganger::{Facing, Faction, LifeState, Position, Stance, Suppressed, Tu},
    injuries::InflictedInjuries,
    metric::CellLevel,
    occupancy::OccupancyGrid,
    pathfinder::{Departure, MoveGrids, PlanningView},
    surface::SurfaceGrid,
    terrain::{
        emplacement::{
            EmplacementEntrySides, EmplacementFacing, EmplacementState, Mounted,
            MountedWeaponEntity, clear_seat,
        },
        entity::TerrainCell,
        floor::FloorCostGrid,
    },
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, OmniscientFog, SquadVisibility, move_fog},
};

/// What move dispatch reads off the actor whose walk it is routing.
#[derive(QueryData)]
pub struct MoverRow {
    pub(super) position: &'static Position,
    pub(super) tu:       &'static Tu,
    pub(super) stance:   &'static Stance,
    pub(super) facing:   &'static Facing,
    pub(super) faction:  &'static Faction,
    pub(super) injuries: Option<&'static InflictedInjuries>,
    pub(super) mounted:  Option<&'static Mounted>,
}

/// What a move reads and writes on the seat a mounted mover rides.
#[derive(QueryData)]
#[query_data(mutable)]
pub struct SeatRow {
    state:  &'static mut EmplacementState,
    cell:   &'static TerrainCell,
    sides:  Option<&'static EmplacementEntrySides>,
    facing: Option<&'static EmplacementFacing>,
    mount:  Option<&'static MountedWeaponEntity>,
}

/// What move dispatch reads off a mover's seat, and what it writes when it commits the walk.
#[derive(SystemParam)]
pub struct MoveCommit<'w, 's> {
    seats:    Query<'w, 's, SeatRow>,
    commands: Commands<'w, 's>,
}

impl MoveCommit<'_, '_> {
    /// Where a route off `from` may leave: only this mover's seat entry cells while it rides one.
    pub(super) fn departure(&self, from: CellLevel, mounted: Option<&Mounted>) -> Departure {
        let Some(seat) = mounted.and_then(Mounted::emplacement) else {
            return Departure::anywhere(from);
        };
        let Ok(row) = self.seats.get(seat) else {
            return Departure::anywhere(from);
        };
        seat_departure(from, **row.cell, row.sides, row.facing)
    }

    /// Clear the seat this mover rides, if it rides one, leaving it standing where it is.
    pub(super) fn vacate(&mut self, mounted: Option<&Mounted>) {
        let Some(seat) = mounted.and_then(Mounted::emplacement) else {
            return;
        };
        let Ok(mut row) = self.seats.get_mut(seat) else {
            return;
        };
        clear_seat(&mut self.commands, seat, &mut row.state, row.mount);
    }

    /// Start this actor walking the route it has just committed to.
    pub(super) fn start_walk(&mut self, actor: Entity, walk: WalkInProgress) {
        self.commands.entity(actor).insert(walk);
    }
}

/// Occupancy, vertical links, floor costs, and tuning a route is planned against.
#[derive(SystemParam)]
pub struct PathfindingGrids<'w> {
    occupancy:   Res<'w, OccupancyGrid>,
    links:       Res<'w, VerticalLinkGraph>,
    floor_costs: Res<'w, FloorCostGrid>,
    tuning:      Res<'w, CombatTuning>,
}

impl PathfindingGrids<'_> {
    /// The tuning every step and every surcharge on this route is priced from.
    pub(super) fn tuning(&self) -> &CombatTuning {
        &self.tuning
    }

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

/// The suppression markers, the cover and the sight a break-away step is judged against.
#[derive(SystemParam)]
pub struct SuppressionGate<'w, 's> {
    suppressed: Query<'w, 's, &'static Suppressed>,
    lives:      Query<'w, 's, &'static LifeState>,
    cover:      Res<'w, CoverLedger>,
    occupancy:  Res<'w, OccupancyGrid>,
    surface:    Res<'w, SurfaceGrid>,
    tuning:     Res<'w, CombatTuning>,
}

impl SuppressionGate<'_, '_> {
    /// Whatever suppression holds this actor, if any holds it at all.
    pub(super) fn on(&self, actor: Entity) -> Option<&Suppressed> {
        self.suppressed.get(actor).ok()
    }

    /// The cover a suppressed mover may break away behind.
    pub(super) fn cover(&self) -> &CoverLedger {
        &self.cover
    }

    /// The grids and tuning a break-away sight probe is flown through.
    pub(super) fn sight(&self) -> SightWorld<'_, impl Fn(Entity) -> bool> {
        SightWorld::new(&self.occupancy, &self.surface, &self.tuning, |entity| {
            self.lives.get(entity).is_ok_and(|life| !*life.is_active())
        })
    }
}
