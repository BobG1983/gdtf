//! Shared headless harness + grid probes for the occupancy-sync tests — the
//! `MinimalPlugins` app builder and the `OccupancyGrid` read-back helpers each
//! concern file reaches via `use super::support::*;`.

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

/// Build a headless app: `MinimalPlugins` (no window / renderer — C1), the
/// full 60×60×8 [`OccupancyGrid`] resource, the [`SurfaceGrid`] (GTW-365 —
/// `sync_destroyed_slab` reads it `ResMut`), and the maintenance plugin (which
/// registers the [`CoverDestroyed`] + [`SlabDestroyed`] messages + the four systems).
pub(super) fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(OccupancyGrid::new());
    // GTW-365: `sync_destroyed_slab` reads `ResMut<SurfaceGrid>` — seed it so the plugin's
    // four-system band validates (these occupancy tests never destroy a slab, but the
    // system's param must resolve).
    app.insert_resource(SurfaceGrid::new());
    app.add_plugins(OccupancyMaintenancePlugin);
    app
}

/// Read the grid resource out of the app world for assertions — `Option` so the
/// test never `unwrap`s (the restriction lints fire in tests too).
pub(super) fn grid_occupant(app: &App, at: CellLevel) -> Option<Entity> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.occupant(&at))
}

/// Read the published occupant band at `at` — `None` if the grid is absent OR no
/// band is published there (the two collapse for the assertions below, which only
/// care about the band's presence/value at a slot known to exist).
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

/// Mark `cell` as a stair tile in the app's [`OccupancyGrid`] resource.
pub(super) fn mark_stair(app: &mut App, cell: CellLevel) {
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.mark_stair_cell(cell);
    }
}
