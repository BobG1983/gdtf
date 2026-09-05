use bevy::{ecs::entity::Entity, prelude::*};
use cobalt_test_utils::{advance_until, press_ui_button};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    acts::open_door_tu_cost,
    cover::HeightBand,
    entity::TerrainCell,
    ganger::Tu,
    openable::{OpenState, OpenableBlocking},
    prelude::{Cell, CellLevel, Faction, Level, Position},
    tuning::CombatTuning,
};
use gdtf_game::test_support::OpenDoorButton;

use super::{actors::*, harness::*};

fn spawn_door_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app
        .world_mut()
        .spawn((
            at(x, y),
            Faction::new(gang),
            gdtf_battle_sim::ganger::Tu::new(100),
            gdtf_battle_sim::ganger::TuMax::new(100),
        ))
        .id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

fn spawn_door(app: &mut App, x: i32, y: i32, state: OpenState) -> Entity {
    app.world_mut()
        .spawn((
            TerrainCell::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
            state,
            OpenableBlocking::new(HeightBand::High),
        ))
        .id()
}

fn open_door_visible(app: &mut App) -> bool {
    visibility::<OpenDoorButton>(app) == Some(Visibility::Visible)
}

fn door_state(app: &App, door: Entity) -> Option<OpenState> {
    app.world().get::<OpenState>(door).copied()
}

#[test]
fn adjacent_closed_door_offers_open_door() {
    let mut app = battle_running_app();
    spawn_door_actor(&mut app, 5, 5, 0);
    spawn_door(&mut app, 6, 6, OpenState::Closed);
    app.update();

    assert!(
        open_door_visible(&mut app),
        "an 8-adjacent CLOSED door must reveal the Open Door button",
    );
    assert!(
        root_visible(&mut app),
        "an offered Open Door must reveal the panel root",
    );
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
    assert!(
        !melee_visible(&mut app),
        "no meleeable neighbour in reach -> the Melee button stays hidden",
    );
    assert!(
        !shove_visible(&mut app),
        "no shovable neighbour in reach -> the Shove button stays hidden",
    );
}

#[test]
fn open_or_non_adjacent_door_does_not_offer_open_door() {
    let mut app = battle_running_app();
    let actor = spawn_door_actor(&mut app, 5, 5, 0);
    spawn_door(&mut app, 5, 6, OpenState::Open);
    spawn_door(&mut app, 30, 30, OpenState::Closed);
    app.update();

    assert!(
        !open_door_visible(&mut app),
        "an adjacent OPEN door + a non-adjacent CLOSED door offer NO open (button hidden)",
    );
    assert!(
        !root_visible(&mut app),
        "with no contextual act offered the panel root stays hidden",
    );

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(actor) {
        *pos = at(29, 30);
    }
    app.update();
    assert!(
        open_door_visible(&mut app),
        "moving the actor next to the CLOSED door reveals the Open Door button (discriminating)",
    );
}

/// What opening one door would charge, taken from the sim's own cost helper.
fn open_door_cost(app: &App) -> Tu {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or_else(|| Tu::new(0), open_door_tu_cost)
}

#[test]
fn a_pool_below_the_open_door_cost_still_offers_the_button_greyed_out() {
    let mut app = battle_running_app();
    let actor = spawn_door_actor(&mut app, 5, 5, 0);
    spawn_door(&mut app, 6, 6, OpenState::Closed);

    let cost = open_door_cost(&app);
    assert!(
        *cost > 0,
        "opening a door must cost something or an unaffordable pool cannot exist",
    );
    set_pool(&mut app, actor, Tu::new(cost.saturating_sub(1)));
    app.update();

    assert!(
        open_door_visible(&mut app),
        "an unaffordable open is still OFFERED — the player sees the act and why it is barred",
    );
    assert!(
        button_greyed::<OpenDoorButton>(&mut app),
        "a pool below the open cost greys the button out",
    );

    set_pool(&mut app, actor, cost);
    app.update();
    app.update();
    assert!(
        !button_greyed::<OpenDoorButton>(&mut app),
        "a pool that covers the cost drops the disabled marker (discriminating)",
    );
}

#[test]
fn pressing_open_door_toggles_the_door_open() {
    let mut app = battle_running_app();
    spawn_door_actor(&mut app, 5, 5, 0);
    let door = spawn_door(&mut app, 6, 6, OpenState::Closed);

    app.update();
    assert!(
        open_door_visible(&mut app),
        "sanity: the Open Door button is offered before the press",
    );
    assert_eq!(
        door_state(&app, door),
        Some(OpenState::Closed),
        "sanity: the door is CLOSED before the press",
    );
    let open_door_btn = the_only::<OpenDoorButton>(
        &mut app,
        "the panel must offer exactly one Open Door button to press",
    );

    press_ui_button(&mut app, open_door_btn);
    advance_until(&mut app, |app| {
        door_state(app, door) == Some(OpenState::Open)
    });
}
