use bevy::prelude::{App, Entity, MinimalPlugins};

use crate::{
    cover::HeightBand,
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::OccupancyMaintenancePlugin,
    surface::SurfaceGrid,
    terrain::{
        entity::{BlocksPathfinding, BlocksVision, TerrainCell},
        openable::{OpenState, OpenableBlocking, OpenableTogglePlugin, SetOpenable},
    },
};

fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.add_plugins(OccupancyMaintenancePlugin);
    app.add_plugins(OpenableTogglePlugin);
    app
}

fn spawn_closed_openable(app: &mut App, at: CellLevel, band: HeightBand) -> Entity {
    app.world_mut()
        .spawn((
            TerrainCell::new(at),
            OpenState::Closed,
            OpenableBlocking::new(band),
            BlocksPathfinding,
            BlocksVision::new(band),
        ))
        .id()
}

fn path_blocked(app: &App, at: CellLevel) -> Option<bool> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .map(|g| *g.is_path_blocked(&at))
}

fn occluder_band(app: &App, at: CellLevel) -> Option<HeightBand> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|g| g.vision_occluder_at(&at))
}

fn open_state(app: &App, entity: Entity) -> Option<OpenState> {
    app.world().get::<OpenState>(entity).copied()
}

fn toggle_and_settle(app: &mut App, request: SetOpenable) {
    app.world_mut().write_message(request);
    app.update();
    app.update();
}

#[test]
fn closed_door_blocks_both_open_clears_both_close_reblocks() {
    let at = key(4, 4, 0);
    let mut app = headless_app();
    let door = spawn_closed_openable(&mut app, at, HeightBand::High);

    app.update();
    assert_eq!(
        open_state(&app, door),
        Some(OpenState::Closed),
        "the door spawns Closed (C1 default)",
    );
    assert_eq!(
        path_blocked(&app, at),
        Some(true),
        "C5(a): a CLOSED Openable blocks the path",
    );
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::High),
        "C5(a): a CLOSED Openable occludes vision (at its closed band)",
    );

    toggle_and_settle(&mut app, SetOpenable::open(door));
    assert_eq!(
        open_state(&app, door),
        Some(OpenState::Open),
        "the toggle flipped the door to Open",
    );
    assert_eq!(
        path_blocked(&app, at),
        Some(false),
        "C5(b): an OPEN Openable no longer blocks the path (the discriminating flip)",
    );
    assert_eq!(
        occluder_band(&app, at),
        None,
        "C5(b): an OPEN Openable no longer occludes vision (the discriminating flip)",
    );

    toggle_and_settle(&mut app, SetOpenable::close(door));
    assert_eq!(
        open_state(&app, door),
        Some(OpenState::Closed),
        "the toggle flipped the door back to Closed",
    );
    assert_eq!(
        path_blocked(&app, at),
        Some(true),
        "C5(c): toggling CLOSED re-blocks the path",
    );
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::High),
        "C5(c): toggling CLOSED re-occludes vision",
    );
}

#[test]
fn closed_openable_slab_blocks_both_despite_kind_default() {
    let at = key(6, 6, 1);
    let mut app = headless_app();
    let hatch = spawn_closed_openable(&mut app, at, HeightBand::High);

    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(true),
        "C5(d): a CLOSED Openable SLAB blocks the path even though a slab does not by default",
    );
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::High),
        "C5(d): a CLOSED Openable SLAB occludes vision even though a slab does not by default",
    );

    toggle_and_settle(&mut app, SetOpenable::open(hatch));
    assert_eq!(
        path_blocked(&app, at),
        Some(false),
        "C5(d): an OPEN Openable SLAB no longer blocks the path",
    );
    assert_eq!(
        occluder_band(&app, at),
        None,
        "C5(d): an OPEN Openable SLAB no longer occludes vision",
    );
}

#[test]
fn close_reinserts_the_recorded_band() {
    let at = key(2, 8, 0);
    let mut app = headless_app();
    let door = spawn_closed_openable(&mut app, at, HeightBand::Mid);
    app.update();
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::Mid),
        "the door occludes at its recorded Mid band when closed",
    );

    toggle_and_settle(&mut app, SetOpenable::open(door));
    assert_eq!(occluder_band(&app, at), None, "open clears the occluder");

    toggle_and_settle(&mut app, SetOpenable::close(door));
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::Mid),
        "C5: closing RE-INSERTS BlocksVision at the OpenableBlocking-recorded Mid band",
    );
}

#[test]
fn toggle_is_idempotent_and_panic_free_on_non_openable() {
    let at = key(1, 1, 0);
    let mut app = headless_app();
    let door = spawn_closed_openable(&mut app, at, HeightBand::High);
    app.update();

    toggle_and_settle(&mut app, SetOpenable::open(door));
    toggle_and_settle(&mut app, SetOpenable::open(door));
    assert_eq!(
        open_state(&app, door),
        Some(OpenState::Open),
        "re-opening an open door stays Open (idempotent)",
    );
    assert_eq!(
        path_blocked(&app, at),
        Some(false),
        "re-opening an open door keeps the path clear",
    );

    let plain: Entity = app.world_mut().spawn(TerrainCell::new(key(9, 9, 0))).id();
    toggle_and_settle(&mut app, SetOpenable::open(plain));
    assert!(
        open_state(&app, plain).is_none(),
        "a non-openable entity gains no OpenState from a stray SetOpenable (panic-free skip)",
    );
}

#[test]
fn toggle_is_deterministic() {
    let at = key(3, 3, 0);
    let run = || {
        let mut app = headless_app();
        let door = spawn_closed_openable(&mut app, at, HeightBand::High);
        app.update();
        toggle_and_settle(&mut app, SetOpenable::open(door));
        toggle_and_settle(&mut app, SetOpenable::close(door));
        (path_blocked(&app, at), occluder_band(&app, at))
    };
    assert_eq!(
        run(),
        run(),
        "C5(e): identical spawn+toggle sequences yield identical surfaces (deterministic)",
    );
    assert_eq!(
        run(),
        (Some(true), Some(HeightBand::High)),
        "C5(e): the settled end state is Closed → blocks both",
    );
}
