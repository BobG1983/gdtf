//! GTW-502 (C4) — the change-detection projection drives the REAL system:
//! `Added<BlocksVision>` occludes a cell on the grid's vision-blocking surface (at the
//! component's band), `Changed<BlocksVision>` re-tunes the band, and
//! `RemovedComponents<BlocksVision>` re-opens it — the occluder surface flips with the
//! component.
//!
//! Drives [`project_vision_blocking`] through the live
//! [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin) chain
//! in a headless `App` (`MinimalPlugins`, no window/renderer — the `path_blocking_test`
//! precedent), `app.update()`-ticking so `Added` / `Changed` / `RemovedComponents` fire for
//! real.

use bevy::prelude::{App, Entity, MinimalPlugins};

use crate::{
    cover::HeightBand,
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::OccupancyMaintenancePlugin,
    surface::SurfaceGrid,
    terrain::entity::{BlocksVision, TerrainCell},
};

/// A `(x, y, level)` cell-key helper.
fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// Build the headless app: `MinimalPlugins` + the full grid resources + the maintenance
/// plugin (which now owns `project_vision_blocking` in its chain). The `SurfaceGrid` is
/// seeded because the plugin's `sync_destroyed_slab` reads it `ResMut`.
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.add_plugins(OccupancyMaintenancePlugin);
    app
}

/// The grid's vision-occluder band at `at` — `None` if the grid resource is absent (kept
/// `Option` so the test never `unwrap`s; the restriction lints fire in tests too).
fn occluder_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.vision_occluder_at(&at))
}

/// C4 — adding the component occludes the cell at its band, removing it re-opens it, re-adding
/// occludes it again: the projection keeps the vision-blocking surface in sync via change
/// detection.
#[test]
fn component_add_remove_flips_vision_blocking() {
    let at = key(3, 4, 0);
    let mut app = headless_app();

    // Spawn a terrain entity carrying the occluder. `Added<BlocksVision>` fires on this
    // insert, so the first update's projection occludes the cell.
    let entity = app
        .world_mut()
        .spawn((TerrainCell::new(at), BlocksVision::new(HeightBand::High)))
        .id();

    // Before any tick the projection has not run — the surface is empty.
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

    // Remove the component — RemovedComponents<BlocksVision> fires; the projection clears
    // the cell next tick (the entity still exists, so its TerrainCell is read for the cell).
    app.world_mut().entity_mut(entity).remove::<BlocksVision>();
    app.update();
    assert_eq!(
        occluder_band(&app, at),
        None,
        "C4: RemovedComponents<BlocksVision> re-opens the cell",
    );

    // Re-add the component — Added fires again; the cell re-occludes.
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

/// C4 — RE-TUNING the band (a `Changed<BlocksVision>` with no add/remove) updates the
/// surface band in place: the height-aware difference from the path SET. The occluder's
/// recorded band follows the component.
#[test]
fn component_band_retune_updates_the_surface() {
    let at = key(5, 5, 0);
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

    // Overwrite the component's band in place — `Changed<BlocksVision>` fires (no Add/Remove).
    // Guarded `if let` (no `expect` — the expect_used lint fires in tests too); the entity was
    // just spawned with the component, so the `else` is unreachable but kept panic-free.
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

/// C4 (despawn) — despawning an occluding terrain entity also re-opens the cell:
/// `RemovedComponents` fires for a despawn too, so the projection clears the surface.
#[test]
fn despawning_occluding_entity_re_opens_cell() {
    let at = key(7, 2, 1);
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

/// C4 (multi) — two occluders on distinct cells both project; removing ONE leaves the other
/// occluding (the projection is per-cell, not all-or-nothing).
#[test]
fn per_cell_projection_is_independent() {
    let a = key(1, 1, 0);
    let b = key(2, 2, 0);
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
