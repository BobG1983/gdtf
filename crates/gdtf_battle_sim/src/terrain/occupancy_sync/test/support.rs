use bevy::prelude::{App, Entity, MinimalPlugins};

use super::super::OccupancyMaintenancePlugin;
use crate::{
    cover::HeightBand,
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
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
    app
}

pub(super) fn grid_occupant(app: &App, at: CellLevel) -> Option<Entity> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant(&at))
}

pub(super) fn grid_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant_band(&at))
}

pub(super) fn cover_destroyed(app: &App, at: CellLevel) -> Option<bool> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .map(|g| *g.is_cover_destroyed(&at))
}

pub(super) fn mark_stair(app: &mut App, cell: CellLevel) {
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.mark_stair_cell(cell);
    }
}
