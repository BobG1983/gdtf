//! System params for the enemy turn: planning grids and outgoing act requests.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, MessageWriter, Res},
};

use crate::{
    acts::{
        EndTurnRequested, FireRequested, MeleeRequested, MoveRequested, OpenDoorRequested,
        ReloadRequested, SetAimingRequested, SetStanceRequested, SightWorld,
    },
    battle::PlayerFaction,
    cover::CoverLedger,
    ganger::Faction,
    march::MarchGrids,
    occupancy::OccupancyGrid,
    pathfinder::MoveGrids,
    surface::SurfaceGrid,
    terrain::floor::FloorCostGrid,
    tuning::CombatTuning,
    turn::ActiveFaction,
    vertical::VerticalLinkGraph,
};

/// The grids and tuning the enemy AI plans sight and routes against.
#[derive(SystemParam)]
pub struct AiPlanningGrids<'w> {
    occupancy:   Res<'w, OccupancyGrid>,
    surface:     Res<'w, SurfaceGrid>,
    cover:       Res<'w, CoverLedger>,
    links:       Res<'w, VerticalLinkGraph>,
    floor_costs: Res<'w, FloorCostGrid>,
    tuning:      Res<'w, CombatTuning>,
}

impl AiPlanningGrids<'_> {
    /// Grids a line of sight marches through.
    pub(super) fn march(&self) -> MarchGrids<'_> {
        MarchGrids {
            occupancy: &self.occupancy,
            surface:   &self.surface,
            cover:     &self.cover,
        }
    }

    /// Grids a route is costed against.
    pub(super) fn routes(&self) -> MoveGrids<'_> {
        MoveGrids {
            occupancy:   &self.occupancy,
            links:       &self.links,
            floor_costs: &self.floor_costs,
            tuning:      &self.tuning,
        }
    }

    /// Combat tuning.
    pub(super) fn tuning(&self) -> &CombatTuning {
        &self.tuning
    }

    /// The cover a suppressed mover has to end behind to break away.
    pub(super) fn cover(&self) -> &CoverLedger {
        &self.cover
    }

    /// The grids and tuning a break-away sight probe is flown through.
    pub(super) fn sight<F: Fn(Entity) -> bool>(&self, is_dead: F) -> SightWorld<'_, F> {
        SightWorld::new(&self.occupancy, &self.surface, &self.tuning, is_dead)
    }
}

/// Whose turn this is, and who the player is.
#[derive(SystemParam)]
pub struct AiTurnSides<'w> {
    active: Res<'w, ActiveFaction>,
    player: Option<Res<'w, PlayerFaction>>,
}

impl AiTurnSides<'_> {
    /// The active enemy faction, when it is not the player's turn.
    pub(super) fn enemy_faction(&self) -> Option<Faction> {
        let player = **self.player.as_deref()?;
        let active = **self.active;
        (active != player).then_some(active)
    }
}

/// The acts one enemy turn requests.
#[derive(SystemParam)]
pub struct AiActRequests<'w> {
    pub(super) fire:      MessageWriter<'w, FireRequested>,
    pub(super) reload:    MessageWriter<'w, ReloadRequested>,
    pub(super) melee:     MessageWriter<'w, MeleeRequested>,
    pub(super) step:      MessageWriter<'w, MoveRequested>,
    pub(super) open_door: MessageWriter<'w, OpenDoorRequested>,
    pub(super) aim:       MessageWriter<'w, SetAimingRequested>,
    pub(super) stance:    MessageWriter<'w, SetStanceRequested>,
    pub(super) end_turn:  MessageWriter<'w, EndTurnRequested>,
}
