//! The screen's picture and the live terrain one reachable search is planned over.

use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::*,
};
use cobalt_mcp_protocol::command::RefusalNote;
use gdtf_battle_presenter::DrawnVitals;
use gdtf_battle_sim::{
    acts::{dismount_surcharge, seat_departure},
    emplacement::{EmplacementEntrySides, EmplacementFacing, Mounted},
    entity::TerrainCell,
    floor::FloorCostGrid,
    ganger::{Faction, LifeState, Tu},
    injuries::{InflictedInjuries, MovementCostFactor},
    occupancy::OccupancyGrid,
    pathfinder::{Departure, MoveGrids, PlanningView, reachable_within},
    prelude::CellLevel,
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
};

use crate::{
    dev::mcp::commands::read::shown::DrawnCell,
    states::running::game::battlescape::inspect_panel::decide::ShownBattle,
};

/// The refusal a call gets while the screen has drawn no occupancy grid to walk.
const NO_GRID: RefusalNote =
    RefusalNote::from_static("the screen has drawn no occupancy grid to search over");

/// The refusal a call gets while the screen has drawn no fog to plan through.
const NO_FOG: RefusalNote =
    RefusalNote::from_static("the screen has drawn no squad fog to plan a route through");

/// The refusal a call gets while nothing names the faction the player commands.
const NO_PLAYER: RefusalNote = RefusalNote::from_static(
    "nothing names the faction the player commands, so no ganger can be checked as theirs",
);

/// The refusal a call gets while the authored terrain and tuning are not loaded.
const NO_TERRAIN: RefusalNote = RefusalNote::from_static(
    "the battle's vertical links, floor costs and tuning are not loaded, so no route can be \
     costed",
);

/// What a reachable search reads off the ganger it is planned for.
#[derive(QueryData)]
pub(super) struct ReachableActorRow {
    /// Where the sprite stands, which lags the sim while an act plays out.
    pub(super) cell:     DrawnCell,
    /// The pool the sim holds, spent only when the screen has drawn no vitals of its own.
    pub(super) live_tu:  &'static Tu,
    /// The pool the screen is drawing, which is the budget while playback carries one.
    pub(super) drawn:    Option<&'static DrawnVitals>,
    /// The gang it fights for, checked against the one the player commands.
    pub(super) faction:  &'static Faction,
    /// Lasting injuries, which make every step cost more.
    pub(super) injuries: Option<&'static InflictedInjuries>,
    /// The seat it rides, which fixes where a route may leave from.
    pub(super) mounted:  Option<&'static Mounted>,
}

/// What a search reads off the seat a mounted ganger rides.
type ReachableSeatRow = (
    &'static TerrainCell,
    Option<&'static EmplacementEntrySides>,
    Option<&'static EmplacementFacing>,
);

/// Every entity row a reachable search looks up by token, by faction, or by seat.
#[derive(SystemParam)]
pub(super) struct ReachableRows<'w, 's> {
    pub(super) actors:   Query<'w, 's, ReachableActorRow>,
    pub(super) tokens:   Query<'w, 's, Option<&'static LifeState>, With<Faction>>,
    pub(super) factions: Query<'w, 's, &'static Faction>,
    pub(super) seats:    Query<'w, 's, ReachableSeatRow>,
}

impl ReachableRows<'_, '_> {
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

/// The live terrain and tuning a route is costed against, each absent until a battle loads.
#[derive(SystemParam)]
pub(super) struct ReachableTerrain<'w> {
    links:       Option<Res<'w, VerticalLinkGraph>>,
    floor_costs: Option<Res<'w, FloorCostGrid>>,
    tuning:      Option<Res<'w, CombatTuning>>,
}

/// The whole picture one search runs over: the screen's shadows and the live terrain.
pub(super) struct ReachableScene<'a> {
    grid:              &'a OccupancyGrid,
    fog:               &'a SquadVisibility,
    links:             &'a VerticalLinkGraph,
    floor_costs:       &'a FloorCostGrid,
    tuning:            &'a CombatTuning,
    /// The faction the player commands.
    pub(super) player: Faction,
}

impl<'a> ReachableScene<'a> {
    /// Borrow the shown grid and fog with the live terrain, or name the piece that is missing.
    pub(super) fn assemble(
        shown: ShownBattle<'a>,
        terrain: &'a ReachableTerrain<'_>,
    ) -> Result<Self, RefusalNote> {
        let grid = shown.grid().ok_or(NO_GRID)?;
        let fog = shown.fog().ok_or(NO_FOG)?;
        let player = shown.player().ok_or(NO_PLAYER)?;
        let (Some(links), Some(floor_costs), Some(tuning)) = (
            terrain.links.as_deref(),
            terrain.floor_costs.as_deref(),
            terrain.tuning.as_deref(),
        ) else {
            return Err(NO_TERRAIN);
        };
        Ok(Self {
            grid,
            fog,
            links,
            floor_costs,
            tuning,
            player: **player,
        })
    }

    /// Every cell the actor's row reaches on this picture, with what each one charges.
    pub(super) fn reached(
        &self,
        rows: &ReachableRows<'_, '_>,
        actor: &ReachableActorRowItem<'_, '_>,
    ) -> Vec<(CellLevel, Tu)> {
        let mover = *actor.faction;
        let planning = PlanningView::new(self.fog, |occupant| {
            relation_to(&rows.factions, mover, occupant)
        });
        reachable_within(
            &rows.departure(actor.cell.at(), actor.mounted),
            actor.drawn.map_or(*actor.live_tu, DrawnVitals::tu),
            dismount_surcharge(actor.mounted, self.tuning),
            MoveGrids {
                occupancy:   self.grid,
                links:       self.links,
                floor_costs: self.floor_costs,
                tuning:      self.tuning,
            },
            actor.injuries.map_or(
                MovementCostFactor::IDENTITY,
                InflictedInjuries::movement_cost_factor,
            ),
            &planning,
        )
    }
}

/// How an occupant relates to the mover: its own squad, or anyone else.
fn relation_to(
    factions: &Query<&'static Faction>,
    mover: Faction,
    occupant: Entity,
) -> FactionRelation {
    match factions.get(occupant) {
        Ok(faction) if *faction == mover => FactionRelation::OwnSquad,
        _ => FactionRelation::Other,
    }
}
