//! Reading the world back: seat state, occupant records, cells and pools.

use bevy::{
    app::App,
    ecs::relationship::Relationship,
    prelude::{Entity, Messages},
};
use gdtf_battle_sim::{
    acts::{MovementOccurred, movement::WalkInProgress},
    cover::{CoverLedger, HeightBand},
    ganger::Position,
    metric::CellLevel,
    occupancy::OccupancyGrid,
    prelude::Tu,
    terrain::emplacement::{EmplacementState, MountedBy, MountedWeaponEntity},
    tuning::CombatTuning,
};

pub(crate) fn state(app: &App, entity: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(entity).copied()
}

pub(crate) fn occupant(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<MountedBy>(entity).map(Relationship::get)
}

/// Where a ganger stands right now.
pub(crate) fn pos_of(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(entity).map(|at| **at)
}

pub(crate) fn mount_entity(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<MountedWeaponEntity>(entity).map(|m| **m)
}

/// Who the occupancy grid says stands on a cell, which [`occupant`] does not answer.
pub(crate) fn grid_occupant(app: &App, at: CellLevel) -> Option<Entity> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|grid| grid.occupant(&at))
}

pub(crate) fn occupant_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&at))
}

/// The band of the cover entry seeded at a cell, which a piece's own def decides.
pub(crate) fn cover_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<CoverLedger>()
        .and_then(|ledger| ledger.peek(&at).map(|entry| entry.height_band))
}

pub(crate) fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

pub(crate) fn enter_tu(app: &App) -> u8 {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or(0, |t| *t.enter_emplacement_tu)
}

pub(crate) fn exit_tu(app: &App) -> u8 {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or(0, |t| *t.exit_emplacement_tu)
}

/// Whether this actor is still walking a committed route.
pub(crate) fn is_walking(app: &App, actor: Entity) -> bool {
    app.world().get::<WalkInProgress>(actor).is_some()
}

/// Every `MovementOccurred` written since the buffer was last drained.
pub(crate) fn drain_movements(app: &mut App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}
