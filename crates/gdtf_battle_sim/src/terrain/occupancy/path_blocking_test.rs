use bevy::prelude::{App, Entity, MinimalPlugins};

use crate::{
    metric::{Cell, CellLevel, Level},
    occupancy::{OccupancyGrid, PathBlocked},
    occupancy_sync::OccupancyMaintenancePlugin,
    surface::SurfaceGrid,
    terrain::entity::{BlocksPathfinding, TerrainCell},
};

fn key(cell: Cell, level: Level) -> CellLevel {
    CellLevel::new(cell, level)
}

fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.add_plugins(OccupancyMaintenancePlugin);
    app
}

fn path_blocked(app: &App, at: CellLevel) -> Option<PathBlocked> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .map(|g| g.is_path_blocked(&at))
}

#[test]
fn marker_add_remove_flips_path_blocking() {
    let at = key(Cell::new(3, 4), Level::new(0));
    let mut app = headless_app();

    let entity = app
        .world_mut()
        .spawn((TerrainCell::new(at), BlocksPathfinding))
        .id();

    assert_eq!(
        path_blocked(&app, at),
        Some(PathBlocked::new(false)),
        "before the first tick, no projection has run — the cell is not path-blocked",
    );

    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(PathBlocked::new(true)),
        "C6(d): Added<BlocksPathfinding> projects the cell as path-blocked",
    );

    app.world_mut()
        .entity_mut(entity)
        .remove::<BlocksPathfinding>();
    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(PathBlocked::new(false)),
        "C6(d): RemovedComponents<BlocksPathfinding> re-opens the cell",
    );

    app.world_mut().entity_mut(entity).insert(BlocksPathfinding);
    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(PathBlocked::new(true)),
        "C6(d): re-adding the marker re-blocks the cell",
    );
}

#[test]
fn despawning_marked_entity_re_opens_cell() {
    let at = key(Cell::new(7, 2), Level::new(1));
    let mut app = headless_app();
    let entity = app
        .world_mut()
        .spawn((TerrainCell::new(at), BlocksPathfinding))
        .id();
    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(PathBlocked::new(true)),
        "the spawned marked entity blocks the cell",
    );

    app.world_mut().entity_mut(entity).despawn();
    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(PathBlocked::new(false)),
        "C6(d·despawn): despawning the marked entity re-opens the cell",
    );
}

#[test]
fn per_cell_projection_is_independent() {
    let a = key(Cell::new(1, 1), Level::new(0));
    let b = key(Cell::new(2, 2), Level::new(0));
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
    assert_eq!(
        path_blocked(&app, a),
        Some(PathBlocked::new(true)),
        "cell a blocked"
    );
    assert_eq!(
        path_blocked(&app, b),
        Some(PathBlocked::new(true)),
        "cell b blocked"
    );

    app.world_mut()
        .entity_mut(ent_a)
        .remove::<BlocksPathfinding>();
    app.update();
    assert_eq!(
        path_blocked(&app, a),
        Some(PathBlocked::new(false)),
        "removing a's marker re-opens a",
    );
    assert_eq!(
        path_blocked(&app, b),
        Some(PathBlocked::new(true)),
        "C6(d·multi): b stays blocked — the projection is per-cell",
    );
}
