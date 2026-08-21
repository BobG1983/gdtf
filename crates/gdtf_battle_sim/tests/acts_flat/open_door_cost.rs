//! Opening a door charges exactly what `open_door_tu_cost` quotes.

use bevy::{
    app::App,
    prelude::{Entity, World},
};
use gdtf_battle_sim::{
    acts::{OpenDoorRequested, can_open_door, open_door_tu_cost},
    cover::HeightBand,
    entity::{BlocksPathfinding, BlocksVision, TerrainCell},
    ganger::TuMax,
    occupancy_sync::OccupancyMaintenancePlugin,
    openable::{OpenState, OpenableBlocking, OpenableTogglePlugin},
    prelude::{Cell, CellLevel, Faction, Level, Position, Tu},
    test_support::SimAppBuilder,
    tuning::CombatTuning,
};

const SEED: u64 = 0x0315_0303_1500_C057;

fn shipped_tuning() -> CombatTuning {
    const SHIPPED: &str = include_str!("../../../../assets/core_tuning/combat.tuning.ron");
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

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
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

fn spawn_closed_door(world: &mut World, at: CellLevel) -> Entity {
    world
        .spawn((
            TerrainCell::new(at),
            OpenState::Closed,
            OpenableBlocking::new(HeightBand::High),
            BlocksPathfinding,
            BlocksVision::new(HeightBand::High),
        ))
        .id()
}

#[test]
fn open_door_charges_exactly_the_quote_and_can_open_door_agrees() {
    let mut app = open_door_app();
    let actor_cell = ground(5, 5);
    let door_cell = ground(6, 5);
    let actor = spawn_actor(app.world_mut(), actor_cell, 100);
    let door = spawn_closed_door(app.world_mut(), door_cell);
    app.update();

    let tuning = shipped_tuning();
    let quoted = open_door_tu_cost(&tuning);
    assert!(
        *quoted > 0,
        "the shipped open-door leaf is a real positive cost"
    );
    assert!(
        *can_open_door(
            Position::new(actor_cell),
            Position::new(door_cell),
            OpenState::Closed,
            &Tu::new(100),
            &tuning,
        ),
        "an adjacent actor with an ample pool may open a closed door",
    );
    assert!(
        !*can_open_door(
            Position::new(actor_cell),
            Position::new(door_cell),
            OpenState::Open,
            &Tu::new(100),
            &tuning,
        ),
        "an already-open door is not openable",
    );
    assert!(
        !*can_open_door(
            Position::new(actor_cell),
            Position::new(ground(9, 5)),
            OpenState::Closed,
            &Tu::new(100),
            &tuning,
        ),
        "a non-adjacent door is not openable",
    );
    assert!(
        !*can_open_door(
            Position::new(actor_cell),
            Position::new(door_cell),
            OpenState::Closed,
            &Tu::new(quoted.saturating_sub(1)),
            &tuning,
        ),
        "a pool one TU below the quote cannot pay for it",
    );

    let before = app.world().get::<Tu>(actor).map(|tu| **tu);
    app.world_mut()
        .write_message(OpenDoorRequested::new(actor, door));
    app.update();
    app.update();

    assert_eq!(
        app.world().get::<OpenState>(door).copied(),
        Some(OpenState::Open),
        "the act opened the door",
    );
    assert_eq!(
        before
            .zip(app.world().get::<Tu>(actor).map(|tu| **tu))
            .map(|(b, a)| b - a),
        Some(*quoted),
        "the act charged exactly what open_door_tu_cost quoted",
    );
}

#[test]
fn two_open_door_requests_in_one_frame_charge_the_door_leaf_once() {
    let mut app = open_door_app();
    let actor_cell = ground(5, 5);
    let door_cell = ground(6, 5);
    let actor = spawn_actor(app.world_mut(), actor_cell, 100);
    let door = spawn_closed_door(app.world_mut(), door_cell);
    app.update();

    let quoted = open_door_tu_cost(&shipped_tuning());
    assert!(
        *quoted > 0,
        "PRECONDITION: the shipped open-door leaf must be positive, or the charge assertion \
         below holds whatever the dispatcher does; it reads {}",
        *quoted,
    );

    let before = app.world().get::<Tu>(actor).map_or(0, |tu| **tu);
    for _ in 0..2 {
        app.world_mut()
            .write_message(OpenDoorRequested::new(actor, door));
    }
    app.update();
    app.update();

    let lost = before.saturating_sub(app.world().get::<Tu>(actor).map_or(0, |tu| **tu));
    assert_eq!(
        lost, *quoted,
        "two open-door requests in one frame must charge the door leaf once: the actor lost \
         {lost} TU against a leaf of {}",
        *quoted,
    );
    assert_eq!(
        app.world().get::<OpenState>(door).copied(),
        Some(OpenState::Open),
        "the one charge must buy the one opening, so the door ends Open against a leaf of {}; it \
         reads {:?}",
        *quoted,
        app.world().get::<OpenState>(door).copied(),
    );
}
