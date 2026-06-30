//! GTW-501 (C6 d) — the change-detection projection drives the REAL systems:
//! `Added<BlocksPathfinding>` blocks a cell on the grid's path-blocking surface, and
//! `RemovedComponents<BlocksPathfinding>` re-opens it — the path result flips with the
//! marker.
//!
//! Drives [`project_path_blocking`] through the live
//! [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin) chain
//! in a headless `App` (`MinimalPlugins`, no window/renderer — the `occupancy_sync` test
//! harness precedent), `app.update()`-ticking so `Added` / `RemovedComponents` fire for
//! real.

use bevy::prelude::{App, Entity, MinimalPlugins};

use crate::{
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::OccupancyMaintenancePlugin,
    surface::SurfaceGrid,
    terrain::entity::{BlocksPathfinding, TerrainCell},
};

/// A `(x, y, level)` cell-key helper.
fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// Build the headless app: `MinimalPlugins` + the full grid resources + the maintenance
/// plugin (which now owns `project_path_blocking` in its chain). The `SurfaceGrid` is
/// seeded because the plugin's `sync_destroyed_slab` reads it `ResMut`.
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.add_plugins(OccupancyMaintenancePlugin);
    app
}

/// Whether the grid reports `at` as PATH-blocked — `None` if the grid resource is absent
/// (kept `Option` so the test never `unwrap`s; the restriction lints fire in tests too).
fn path_blocked(app: &App, at: CellLevel) -> Option<bool> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .map(|g| g.is_path_blocked(&at))
}

/// C6(d) — adding the marker blocks the cell, removing it re-opens it, re-adding blocks it
/// again: the projection keeps the path-blocking surface in sync via change detection.
#[test]
fn marker_add_remove_flips_path_blocking() {
    let at = key(3, 4, 0);
    let mut app = headless_app();

    // Spawn a terrain entity carrying the marker. `Added<BlocksPathfinding>` fires on this
    // insert, so the first update's projection blocks the cell.
    let entity = app
        .world_mut()
        .spawn((TerrainCell::new(at), BlocksPathfinding))
        .id();

    // Before any tick the projection has not run — the surface is empty.
    assert_eq!(
        path_blocked(&app, at),
        Some(false),
        "before the first tick, no projection has run — the cell is not path-blocked",
    );

    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(true),
        "C6(d): Added<BlocksPathfinding> projects the cell as path-blocked",
    );

    // Remove the marker — RemovedComponents<BlocksPathfinding> fires; the projection clears
    // the cell next tick (the entity still exists, so its TerrainCell is read for the cell).
    app.world_mut()
        .entity_mut(entity)
        .remove::<BlocksPathfinding>();
    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(false),
        "C6(d): RemovedComponents<BlocksPathfinding> re-opens the cell",
    );

    // Re-add the marker — Added fires again; the cell re-blocks.
    app.world_mut().entity_mut(entity).insert(BlocksPathfinding);
    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(true),
        "C6(d): re-adding the marker re-blocks the cell",
    );
}

/// C6(d·despawn) — despawning a marked terrain entity also re-opens the cell:
/// `RemovedComponents` fires for a despawn too, so the projection clears the surface.
/// (Belt-and-braces: the runtime removal path the cover-smash flow could later use.)
#[test]
fn despawning_marked_entity_re_opens_cell() {
    let at = key(7, 2, 1);
    let mut app = headless_app();
    let entity = app
        .world_mut()
        .spawn((TerrainCell::new(at), BlocksPathfinding))
        .id();
    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(true),
        "the spawned marked entity blocks the cell",
    );

    app.world_mut().entity_mut(entity).despawn();
    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(false),
        "C6(d·despawn): despawning the marked entity re-opens the cell",
    );
}

/// C6(d·multi) — two markers on distinct cells both project; removing ONE leaves the other
/// blocked (the projection is per-cell, not all-or-nothing).
#[test]
fn per_cell_projection_is_independent() {
    let a = key(1, 1, 0);
    let b = key(2, 2, 0);
    let mut app = headless_app();
    let ent_a = app
        .world_mut()
        .spawn((TerrainCell::new(a), BlocksPathfinding))
        .id();
    let _ent_b: Entity = app
        .world_mut()
        .spawn((TerrainCell::new(b), BlocksPathfinding))
        .id();
    app.update();
    assert_eq!(path_blocked(&app, a), Some(true), "cell a blocked");
    assert_eq!(path_blocked(&app, b), Some(true), "cell b blocked");

    app.world_mut()
        .entity_mut(ent_a)
        .remove::<BlocksPathfinding>();
    app.update();
    assert_eq!(
        path_blocked(&app, a),
        Some(false),
        "removing a's marker re-opens a",
    );
    assert_eq!(
        path_blocked(&app, b),
        Some(true),
        "C6(d·multi): b stays blocked — the projection is per-cell",
    );
}
