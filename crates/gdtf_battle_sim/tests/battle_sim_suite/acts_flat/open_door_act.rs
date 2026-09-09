//! Open-door act: adjacency, TU cost, occupancy, and rejection paths.

use bevy::{
    app::App,
    prelude::{Entity, World},
};
use gdtf_battle_sim::{
    acts::OpenDoorRequested,
    cover::HeightBand,
    entity::{BlocksPathfinding, BlocksVision, TerrainCell},
    ganger::TuMax,
    occupancy_sync::OccupancyMaintenancePlugin,
    openable::{OpenState, OpenableBlocking, OpenableTogglePlugin},
    prelude::{Cell, CellLevel, Faction, Level, OccupancyGrid, Position, Tu},
    test_support::SimAppBuilder,
    tuning::CombatTuning,
};

const SEED: u64 = 0x0315_0303_1500_D00D;

fn shipped_tuning() -> CombatTuning {
    const SHIPPED: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/core_tuning/combat.tuning.ron"
    ));
    ron::from_str::<CombatTuning>(SHIPPED).unwrap_or_default()
}

fn open_door_app() -> App {
    let mut app = SimAppBuilder::new()
        .with_seed(SEED)
        .with_acts()
        .with_tuning(shipped_tuning())
        .build();
    app.add_plugins(OpenableTogglePlugin)
        .add_plugins(OccupancyMaintenancePlugin);
    app
}

fn spawn_actor(world: &mut World, at: CellLevel, tu: u8) -> Entity {
    world
        .spawn((
            Position::new(at),
            Faction::new(0),
            Tu::new(tu),
            TuMax::new(tu),
        ))
        .id()
}

fn spawn_closed_door(world: &mut World, at: CellLevel, band: HeightBand) -> Entity {
    world
        .spawn((
            TerrainCell::new(at),
            OpenState::Closed,
            OpenableBlocking::new(band),
            BlocksPathfinding,
            BlocksVision::new(band),
        ))
        .id()
}

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn door_state(app: &App, door: Entity) -> Option<OpenState> {
    app.world().get::<OpenState>(door).copied()
}

fn tu_of(app: &App, actor: Entity) -> u8 {
    *app.world().get::<Tu>(actor).copied().unwrap_or(Tu::new(0))
}

fn open_door_tu(app: &App) -> u8 {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or(0, |t| *t.open_door_tu)
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

fn open_and_settle(app: &mut App, actor: Entity, door: Entity) {
    app.world_mut()
        .write_message(OpenDoorRequested::new(actor, door));
    app.update();
    app.update();
}

#[test]
fn valid_request_toggles_closed_door_open_and_charges_exactly_open_door_tu() {
    let mut app = open_door_app();
    let actor = spawn_actor(app.world_mut(), ground(5, 5), 100);
    let door = spawn_closed_door(app.world_mut(), ground(6, 5), HeightBand::High);
    app.update();
    let cost = open_door_tu(&app);
    assert!(
        cost > 0,
        "the shipped open_door_tu leaf is a real positive cost"
    );
    let tu_before = tu_of(&app, actor);
    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Closed),
        "the door starts Closed",
    );

    open_and_settle(&mut app, actor, door);

    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Open),
        "(a): the open-door act toggled the CLOSED door to Open",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before - cost,
        "(a): the act charged EXACTLY the OpenDoorTu leaf off the actor's pool",
    );
}

#[test]
fn diagonally_adjacent_actor_opens_the_door() {
    let mut app = open_door_app();
    let actor = spawn_actor(app.world_mut(), ground(5, 5), 100);
    let door = spawn_closed_door(app.world_mut(), ground(6, 6), HeightBand::High);
    app.update();

    open_and_settle(&mut app, actor, door);

    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Open),
        "(a): a diagonally-8-adjacent actor opens the door (Chebyshev-1 includes diagonals)",
    );
}

#[test]
fn non_adjacent_actor_is_rejected_no_toggle_no_charge() {
    let mut app = open_door_app();
    let actor = spawn_actor(app.world_mut(), ground(5, 5), 100);
    let door = spawn_closed_door(app.world_mut(), ground(7, 5), HeightBand::High);
    app.update();
    let tu_before = tu_of(&app, actor);

    open_and_settle(&mut app, actor, door);

    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Closed),
        "(b): a NON-adjacent actor does not open the door (the 8-adjacency gate held)",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "(b): a rejected (non-adjacent) open-door spends NO TU",
    );
}

#[test]
fn insufficient_tu_is_rejected_no_toggle_no_charge() {
    let mut app = open_door_app();
    let broke = open_door_tu(&app).saturating_sub(1);
    let actor = spawn_actor(app.world_mut(), ground(5, 5), broke);
    let door = spawn_closed_door(app.world_mut(), ground(6, 5), HeightBand::High);
    app.update();
    let tu_before = tu_of(&app, actor);

    open_and_settle(&mut app, actor, door);

    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Closed),
        "(b): an actor that cannot afford OpenDoorTu does not open the door (the afford gate held)",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "(b): a rejected (unaffordable) open-door spends NO TU",
    );
}

#[test]
fn already_open_door_is_a_noop_no_charge() {
    let mut app = open_door_app();
    let actor = spawn_actor(app.world_mut(), ground(5, 5), 100);
    let door = spawn_closed_door(app.world_mut(), ground(6, 5), HeightBand::High);
    app.update();

    open_and_settle(&mut app, actor, door);
    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Open),
        "the door is open after the first act",
    );
    let tu_after_open = tu_of(&app, actor);

    open_and_settle(&mut app, actor, door);
    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Open),
        "(b): an already-open door stays Open (the closed-gate held — the button only opens)",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_after_open,
        "(b): opening an already-open door spends NO further TU",
    );
}

#[test]
fn opening_the_door_clears_path_and_vision_via_openable_toggle_downstream() {
    let at = ground(6, 5);
    let mut app = open_door_app();
    let actor = spawn_actor(app.world_mut(), ground(5, 5), 100);
    let door = spawn_closed_door(app.world_mut(), at, HeightBand::High);
    app.update();
    assert_eq!(
        path_blocked(&app, at),
        Some(true),
        "(c): a CLOSED door blocks the path before opening",
    );
    assert_eq!(
        occluder_band(&app, at),
        Some(HeightBand::High),
        "(c): a CLOSED door occludes vision before opening",
    );

    open_and_settle(&mut app, actor, door);
    app.update();

    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Open),
        "(c): the act opened the door",
    );
    assert_eq!(
        path_blocked(&app, at),
        Some(false),
        "(c): an OPEN door no longer blocks the path",
    );
    assert_eq!(
        occluder_band(&app, at),
        None,
        "(c): an OPEN door no longer occludes vision",
    );
}

#[test]
fn non_openable_target_is_skipped_panic_free() {
    let mut app = open_door_app();
    let actor = spawn_actor(app.world_mut(), ground(5, 5), 100);
    let plain: Entity = app.world_mut().spawn(TerrainCell::new(ground(6, 5))).id();
    app.update();
    let tu_before = tu_of(&app, actor);

    open_and_settle(&mut app, actor, plain);

    assert!(
        door_state(&app, plain).is_none(),
        "(b): a non-openable target gains no OpenState from a stray OpenDoorRequested",
    );
    assert_eq!(
        tu_of(&app, actor),
        tu_before,
        "(b): an open-door over a non-openable target spends NO TU (panic-free skip)",
    );
}
