use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::{EnterEmplacementButton, ExitEmplacementButton};
use gdtf_battle_input::{SelectedShooter, contextual::ContextualActSystems};
use gdtf_battle_sim::{
    acts::{ExitEmplacementRequested, enter_emplacement_tu_cost},
    emplacement::{EmplacementOccupant, EmplacementState},
    entity::TerrainCell,
    ganger::Tu,
    prelude::{Cell, CellLevel, Faction, Level, Position},
    tuning::CombatTuning,
};
use gdtf_test_utils::{MessageProbe, advance_until, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};

fn spawn_emplacement_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
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

fn spawn_emplacement(
    app: &mut App,
    x: i32,
    y: i32,
    state: EmplacementState,
    occupant: Option<Entity>,
) -> Entity {
    let mut entity = app.world_mut().spawn((
        TerrainCell::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
        state,
    ));
    if let Some(occupant) = occupant {
        entity.insert(EmplacementOccupant::new(occupant));
    }
    entity.id()
}

fn enter_emplacement_visible(app: &mut App) -> bool {
    visibility::<EnterEmplacementButton>(app) == Some(Visibility::Visible)
}

fn exit_emplacement_visible(app: &mut App) -> bool {
    visibility::<ExitEmplacementButton>(app) == Some(Visibility::Visible)
}

fn emplacement_state(app: &App, emplacement: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(emplacement).copied()
}

#[test]
fn adjacent_vacant_emplacement_offers_enter() {
    let mut app = battle_running_app();
    spawn_emplacement_actor(&mut app, 5, 5, 0);
    spawn_emplacement(&mut app, 6, 6, EmplacementState::Vacant, None);
    app.update();

    assert!(
        enter_emplacement_visible(&mut app),
        "an 8-adjacent VACANT emplacement must reveal the Enter button",
    );
    assert!(
        root_visible(&mut app),
        "an offered Enter must reveal the panel root",
    );
    assert!(
        !exit_emplacement_visible(&mut app),
        "the selection is not manning an emplacement -> the Exit button stays hidden",
    );
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
    assert!(
        !shove_visible(&mut app),
        "no shovable neighbour in reach -> the Shove button stays hidden",
    );
}

#[test]
fn occupied_or_non_adjacent_emplacement_does_not_offer_enter() {
    let mut app = battle_running_app();
    let actor = spawn_emplacement_actor(&mut app, 5, 5, 0);
    let other = app.world_mut().spawn_empty().id();
    spawn_emplacement(&mut app, 5, 6, EmplacementState::Occupied, Some(other));
    spawn_emplacement(&mut app, 30, 30, EmplacementState::Vacant, None);
    app.update();

    assert!(
        !enter_emplacement_visible(&mut app),
        "an adjacent OCCUPIED emplacement + a non-adjacent VACANT one offer NO enter (button hidden)",
    );

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(actor) {
        *pos = at(29, 30);
    }
    app.update();
    assert!(
        enter_emplacement_visible(&mut app),
        "moving the actor next to the VACANT emplacement reveals the Enter button (discriminating)",
    );
}

#[test]
fn exit_offered_only_to_the_occupant() {
    let mut app = battle_running_app();
    let actor = spawn_emplacement_actor(&mut app, 5, 5, 0);
    spawn_emplacement(&mut app, 5, 5, EmplacementState::Occupied, Some(actor));
    app.update();

    assert!(
        exit_emplacement_visible(&mut app),
        "the selection manning an emplacement must reveal the Exit button",
    );
    assert!(
        root_visible(&mut app),
        "an offered Exit must reveal the panel root",
    );
    assert!(
        !enter_emplacement_visible(&mut app),
        "the only emplacement is OCCUPIED (by the selection) -> the Enter button stays hidden",
    );

    let other = app.world_mut().spawn_empty().id();
    let emplacements = all_with::<EmplacementState>(&mut app);
    if let [emplacement] = emplacements.as_slice() {
        app.world_mut()
            .entity_mut(*emplacement)
            .insert(EmplacementOccupant::new(other));
    }
    app.update();
    assert!(
        !exit_emplacement_visible(&mut app),
        "an emplacement occupied by SOMEONE ELSE offers the selection no Exit (occupant-only)",
    );
}

#[test]
fn a_vacant_seat_still_naming_the_actor_as_occupant_offers_no_exit() {
    let mut app = battle_running_app();
    let actor = spawn_emplacement_actor(&mut app, 5, 5, 0);
    let emplacement = spawn_emplacement(&mut app, 5, 5, EmplacementState::Vacant, Some(actor));
    app.update();

    assert!(
        !exit_emplacement_visible(&mut app),
        "a VACANT seat is nobody's to leave, whatever occupant it still names -> no Exit",
    );

    if let Ok(mut entity) = app.world_mut().get_entity_mut(emplacement) {
        entity.insert(EmplacementState::Occupied);
    }
    app.update();
    assert!(
        exit_emplacement_visible(&mut app),
        "flipping the same seat to OCCUPIED reveals the Exit button (discriminating)",
    );
}

/// What taking one emplacement would charge, taken from the sim's own cost helper.
fn enter_cost(app: &App) -> Tu {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or_else(|| Tu::new(0), enter_emplacement_tu_cost)
}

#[test]
fn a_pool_below_the_enter_cost_still_offers_the_button_greyed_out() {
    let mut app = battle_running_app();
    let actor = spawn_emplacement_actor(&mut app, 5, 5, 0);
    spawn_emplacement(&mut app, 6, 6, EmplacementState::Vacant, None);

    let cost = enter_cost(&app);
    assert!(
        *cost > 0,
        "entering must cost something or an unaffordable pool cannot exist",
    );
    set_pool(&mut app, actor, Tu::new(cost.saturating_sub(1)));
    app.update();

    assert!(
        enter_emplacement_visible(&mut app),
        "an unaffordable enter is still OFFERED — the player sees the act and why it is barred",
    );
    assert!(
        button_greyed::<EnterEmplacementButton>(&mut app),
        "a pool below the enter cost greys the button out",
    );

    set_pool(&mut app, actor, cost);
    app.update();
    app.update();
    assert!(
        !button_greyed::<EnterEmplacementButton>(&mut app),
        "a pool that covers the cost drops the disabled marker (discriminating)",
    );
}

#[test]
fn pressing_enter_mans_the_emplacement() {
    let mut app = battle_running_app();
    spawn_emplacement_actor(&mut app, 5, 5, 0);
    let emplacement = spawn_emplacement(&mut app, 6, 6, EmplacementState::Vacant, None);

    app.update();
    assert!(
        enter_emplacement_visible(&mut app),
        "sanity: the Enter button is offered before the press",
    );
    assert_eq!(
        emplacement_state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "sanity: the emplacement is VACANT before the press",
    );
    let enter_btn = the_only::<EnterEmplacementButton>(
        &mut app,
        "the panel must offer exactly one Enter button to press",
    );

    press_ui_button(&mut app, enter_btn);
    advance_until(&mut app, |app| {
        emplacement_state(app, emplacement) == Some(EmplacementState::Occupied)
    });

    let occupant = app
        .world()
        .get::<EmplacementOccupant>(emplacement)
        .map(|occupant| **occupant);
    let selected = app
        .world()
        .get_resource::<SelectedShooter>()
        .and_then(|selection| **selection);
    assert_eq!(
        occupant, selected,
        "the recorded EmplacementOccupant is the acting selection",
    );
}

fn add_exit_probe(app: &mut App) {
    app.init_resource::<MessageProbe<ExitEmplacementRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<ExitEmplacementRequested>.after(ContextualActSystems::Drain),
    );
}

fn exit_requests(app: &App) -> Vec<ExitEmplacementRequested> {
    probed::<ExitEmplacementRequested>(app)
}

#[test]
fn pressing_exit_emits_exit_emplacement_requested_for_manned_mount() {
    let mut app = battle_running_app();
    add_exit_probe(&mut app);
    let actor = spawn_emplacement_actor(&mut app, 5, 5, 0);
    let emplacement = spawn_emplacement(&mut app, 5, 5, EmplacementState::Occupied, Some(actor));

    app.update();
    assert!(
        exit_emplacement_visible(&mut app),
        "sanity: the Exit button is offered before the press",
    );
    let exit_btn = the_only::<ExitEmplacementButton>(
        &mut app,
        "the panel must offer exactly one Exit button to press",
    );

    press_ui_button(&mut app, exit_btn);
    app.update();

    let emitted = exit_requests(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Exit while manning must emit exactly one ExitEmplacementRequested",
    );
    assert_eq!(emitted[0].actor, actor, "the actor is the SelectedShooter");
    assert_eq!(
        emitted[0].emplacement, emplacement,
        "the emplacement is the carried manned mount",
    );
}
