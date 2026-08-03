use bevy::prelude::{App, Entity, MinimalPlugins};

use crate::{
    cover::HeightBand,
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::OccupancyMaintenancePlugin,
    surface::SurfaceGrid,
    terrain::entity::{BlocksVision, TerrainCell},
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

fn occluder_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.vision_occluder_at(&at))
}

#[test]
fn component_add_remove_flips_vision_blocking() {
    let at = key(Cell::new(3, 4), Level::new(0));
    let mut app = headless_app();

    let entity = app
        .world_mut()
        .spawn((TerrainCell::new(at), BlocksVision::new(HeightBand::High)))
        .id();

    assert_eq!(
        occluder_band(&app, at),
        None,
        "before the first tick, no projection has run — the cell is not vision-occluding",
    );

    app.update();
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::High),
        "C4: Added<BlocksVision> projects the cell as vision-occluding at its band",
    );

    app.world_mut().entity_mut(entity).remove::<BlocksVision>();
    app.update();
    assert_eq!(
        occluder_band(&app, at),
        None,
        "C4: RemovedComponents<BlocksVision> re-opens the cell",
    );

    app.world_mut()
        .entity_mut(entity)
        .insert(BlocksVision::new(HeightBand::High));
    app.update();
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::High),
        "C4: re-adding the component re-occludes the cell",
    );
}

#[test]
fn component_band_retune_updates_the_surface() {
    let at = key(Cell::new(5, 5), Level::new(0));
    let mut app = headless_app();
    let entity = app
        .world_mut()
        .spawn((TerrainCell::new(at), BlocksVision::new(HeightBand::Low)))
        .id();
    app.update();
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::Low),
        "the spawned occluder records its initial Low band",
    );

    if let Some(mut blocks) = app.world_mut().entity_mut(entity).get_mut::<BlocksVision>() {
        *blocks = BlocksVision::new(HeightBand::High);
    }
    app.update();
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::High),
        "C4: a Changed<BlocksVision> band re-tune updates the surface band in place (Low → High)",
    );
}

#[test]
fn despawning_occluding_entity_re_opens_cell() {
    let at = key(Cell::new(7, 2), Level::new(1));
    let mut app = headless_app();
    let entity = app
        .world_mut()
        .spawn((TerrainCell::new(at), BlocksVision::new(HeightBand::Mid)))
        .id();
    app.update();
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::Mid),
        "the spawned occluding entity occludes the cell",
    );

    app.world_mut().entity_mut(entity).despawn();
    app.update();
    assert_eq!(
        occluder_band(&app, at),
        None,
        "C4 (despawn): despawning the occluding entity re-opens the cell",
    );
}

#[test]
fn per_cell_projection_is_independent() {
    let a = key(Cell::new(1, 1), Level::new(0));
    let b = key(Cell::new(2, 2), Level::new(0));
    let mut app = headless_app();
    let ent_a = app
        .world_mut()
        .spawn((TerrainCell::new(a), BlocksVision::new(HeightBand::High)))
        .id();
    let _ent_b: Entity = app
        .world_mut()
        .spawn((TerrainCell::new(b), BlocksVision::new(HeightBand::High)))
        .id();
    app.update();
    assert_eq!(
        occluder_band(&app, a),
        Some(HeightBand::High),
        "cell a occludes"
    );
    assert_eq!(
        occluder_band(&app, b),
        Some(HeightBand::High),
        "cell b occludes"
    );

    app.world_mut().entity_mut(ent_a).remove::<BlocksVision>();
    app.update();
    assert_eq!(
        occluder_band(&app, a),
        None,
        "removing a's component re-opens a",
    );
    assert_eq!(
        occluder_band(&app, b),
        Some(HeightBand::High),
        "C4 (multi): b stays occluding — the projection is per-cell",
    );
}
