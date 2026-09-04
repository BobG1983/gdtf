//! The world rows and resources one cost preview is priced from.

use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::*,
};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    cover::CoverLedger,
    emplacement::{
        EmplacementEntrySides, EmplacementFacing, EmplacementState, EnteredFrom, Mounted, MountedBy,
    },
    entity::TerrainCell,
    floor::FloorCostGrid,
    ganger::{Aiming, Facing, Faction, LifeState, Position, Stance, Suppressed, Tu, TuMax},
    injuries::InflictedInjuries,
    magazine::Magazine,
    occupancy::OccupancyGrid,
    openable::OpenState,
    pathfinder::MoveGrids,
    surface::SurfaceGrid,
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
    weapon::{
        FightMode, FireMode, Handedness, MeleeWeapon, MountedWeapon, TrajectoryStyle, WieldedBy,
        Wields,
    },
};

/// Everything a cost reads off the acting ganger.
#[derive(QueryData)]
pub(super) struct ActorRow {
    pub(super) position: &'static Position,
    pub(super) facing:   &'static Facing,
    pub(super) stance:   &'static Stance,
    pub(super) aiming:   &'static Aiming,
    pub(super) tu:       &'static Tu,
    pub(super) tu_max:   &'static TuMax,
    pub(super) faction:  &'static Faction,
    pub(super) life:     &'static LifeState,
    pub(super) injuries: Option<&'static InflictedInjuries>,
    pub(super) pinned:   Option<&'static Suppressed>,
    pub(super) mounted:  Option<&'static Mounted>,
}

/// Everything a cost reads off the wielded ranged weapon.
#[derive(QueryData)]
pub(super) struct GunRow {
    pub(super) modes:      &'static FireMode,
    pub(super) magazine:   &'static Magazine,
    pub(super) handedness: &'static Handedness,
    pub(super) trajectory: &'static TrajectoryStyle,
}

/// The geometry, allegiance and life a reach check reads off a target ganger.
pub(super) type TargetRow = (&'static Position, &'static Faction, &'static LifeState);

/// The state, cell, occupant, remembered cell, entry sides and facing an emplacement act reads.
#[derive(QueryData)]
pub(super) struct EmplacementRow {
    pub(super) state:   &'static EmplacementState,
    pub(super) cell:    &'static TerrainCell,
    pub(super) held_by: Option<&'static MountedBy>,
    pub(super) entered: Option<&'static EnteredFrom>,
    pub(super) sides:   Option<&'static EmplacementEntrySides>,
    pub(super) facing:  Option<&'static EmplacementFacing>,
}

/// The battle resources every priced act needs, once all of them are loaded.
pub(super) struct LoadedWorld<'a> {
    /// Tuning every TU cost is priced from.
    pub(super) tuning:  &'a CombatTuning,
    /// The faction the player commands.
    pub(super) player:  Faction,
    /// Occupancy, links, floor costs and tuning a route is costed against.
    pub(super) grids:   MoveGrids<'a>,
    /// The squad fog a player route is planned through.
    pub(super) squad:   &'a SquadVisibility,
    /// The cover a suppressed mover would have to break away behind.
    pub(super) cover:   &'a CoverLedger,
    /// Floor and slab heights a line-of-sight probe is marched over.
    pub(super) surface: &'a SurfaceGrid,
}

/// The sim resources a cost preview reads, each absent until a battle is loaded.
#[derive(SystemParam)]
pub(super) struct CostWorld<'w> {
    tuning:      Option<Res<'w, CombatTuning>>,
    player:      Option<Res<'w, PlayerFaction>>,
    occupancy:   Option<Res<'w, OccupancyGrid>>,
    links:       Option<Res<'w, VerticalLinkGraph>>,
    floor_costs: Option<Res<'w, FloorCostGrid>>,
    squad:       Option<Res<'w, SquadVisibility>>,
    cover:       Option<Res<'w, CoverLedger>>,
    surface:     Option<Res<'w, SurfaceGrid>>,
}

impl CostWorld<'_> {
    /// Every resource a price needs, or nothing when the battle runtime is not up.
    pub(super) fn loaded(&self) -> Option<LoadedWorld<'_>> {
        let tuning = self.tuning.as_deref()?;
        Some(LoadedWorld {
            tuning,
            player: **self.player.as_deref()?,
            grids: MoveGrids {
                occupancy: self.occupancy.as_deref()?,
                links: self.links.as_deref()?,
                floor_costs: self.floor_costs.as_deref()?,
                tuning,
            },
            squad: self.squad.as_deref()?,
            cover: self.cover.as_deref()?,
            surface: self.surface.as_deref()?,
        })
    }
}

/// Every entity row a priced act looks up by token or by what the actor wields.
#[derive(SystemParam)]
pub(super) struct CostRows<'w, 's> {
    pub(super) actors:       Query<'w, 's, ActorRow>,
    pub(super) tokens:       Query<'w, 's, Option<&'static LifeState>, With<Faction>>,
    pub(super) factions:     Query<'w, 's, &'static Faction>,
    pub(super) targets:      Query<'w, 's, TargetRow>,
    pub(super) stances:      Query<'w, 's, &'static Stance>,
    pub(super) wields:       Query<'w, 's, &'static Wields>,
    pub(super) guns:         Query<'w, 's, GunRow, With<WieldedBy>>,
    pub(super) fights:       Query<'w, 's, &'static FightMode, With<WieldedBy>>,
    pub(super) melee:        Query<'w, 's, (), With<MeleeWeapon>>,
    pub(super) mounted:      Query<'w, 's, (), With<MountedWeapon>>,
    pub(super) doors:        Query<'w, 's, (&'static OpenState, &'static TerrainCell)>,
    pub(super) emplacements: Query<'w, 's, EmplacementRow>,
}
