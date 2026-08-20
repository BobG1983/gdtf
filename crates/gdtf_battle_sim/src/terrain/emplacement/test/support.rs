//! Fixtures the emplacement toggle's unit tests share.

use bevy::{
    ecs::relationship::Relationship,
    prelude::{App, Entity, MinimalPlugins},
};

use crate::{
    cover::HeightBand,
    ganger::{Position, StanceKind},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::OccupancyMaintenancePlugin,
    surface::SurfaceGrid,
    terrain::{
        emplacement::{
            EmplacementState, EmplacementTogglePlugin, EnteredFrom, MountedBy, SetEmplacement,
        },
        entity::TerrainCell,
    },
    test_support::GangerEntityBuilder,
};

pub(super) fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

pub(super) fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.add_plugins(OccupancyMaintenancePlugin);
    app.add_plugins(EmplacementTogglePlugin);
    app
}

pub(super) fn spawn_vacant_emplacement(app: &mut App, at: CellLevel) -> Entity {
    app.world_mut()
        .spawn((TerrainCell::new(at), EmplacementState::Vacant))
        .id()
}

/// An emplacement already manned, with no record of where its occupant came from.
pub(super) fn spawn_occupied_emplacement(app: &mut App, at: CellLevel, occupant: Entity) -> Entity {
    app.world_mut()
        .spawn((
            TerrainCell::new(at),
            EmplacementState::Occupied,
            MountedBy::new(occupant),
        ))
        .id()
}

/// A ganger with a stance and NO position, which the toggle's position write cannot reach.
pub(super) fn stanced_ganger(app: &mut App, stance: StanceKind) -> Entity {
    GangerEntityBuilder::new()
        .stance(stance)
        .spawn(app.world_mut())
}

/// A ganger with a stance, standing on a cell of its own.
pub(super) fn stanced_ganger_at(app: &mut App, stance: StanceKind, at: CellLevel) -> Entity {
    GangerEntityBuilder::new()
        .stance(stance)
        .at(at)
        .spawn(app.world_mut())
}

pub(super) fn state(app: &App, entity: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(entity).copied()
}

pub(super) fn occupant(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<MountedBy>(entity).map(Relationship::get)
}

/// The cell an emplacement remembers its occupant entered from.
pub(super) fn entered_from(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<EnteredFrom>(entity).map(|from| **from)
}

/// Where a ganger stands right now.
pub(super) fn position_of(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(entity).map(|at| **at)
}

pub(super) fn occupant_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&at))
}

/// Who the OCCUPANCY GRID says is standing on a cell, which `occupant` above does not answer.
pub(super) fn grid_occupant(app: &App, at: CellLevel) -> Option<Entity> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant(&at))
}

pub(super) fn toggle_and_settle(app: &mut App, request: SetEmplacement) {
    app.world_mut().write_message(request);
    app.update();
    app.update();
}
