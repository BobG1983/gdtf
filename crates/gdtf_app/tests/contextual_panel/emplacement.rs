//! Taking a mount: which seats offer the Enter button, what it costs, and what pressing it does.

use bevy::{
    ecs::{entity::Entity, relationship::Relationship},
    prelude::*,
};
use gdtf_app::test_support::EnterEmplacementButton;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    acts::enter_emplacement_tu_cost,
    emplacement::{EmplacementState, MountedBy},
    ganger::Tu,
    prelude::Position,
    tuning::CombatTuning,
};
use gdtf_test_utils::{advance_until, press_ui_button};

use super::{actors::*, harness::*};

fn emplacement_state(app: &App, emplacement: Entity) -> Option<EmplacementState> {
    app.world().get::<EmplacementState>(emplacement).copied()
}

#[test]
fn adjacent_vacant_emplacement_offers_enter() {
    let mut app = battle_running_app();
    spawn_emplacement_actor(&mut app, 5, 5, 0);
    spawn_emplacement(
        &mut app,
        6,
        5,
        EmplacementState::Vacant,
        None,
        Some(cardinal_sides()),
        Some(cardinal_facing()),
    );
    app.update();

    assert!(
        enter_emplacement_visible(&mut app),
        "the ganger stands on the seat's west entry side, so the VACANT emplacement must reveal \
         the Enter button",
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
    spawn_emplacement(
        &mut app,
        5,
        6,
        EmplacementState::Occupied,
        Some(other),
        Some(cardinal_sides()),
        Some(cardinal_facing()),
    );
    spawn_emplacement(
        &mut app,
        30,
        30,
        EmplacementState::Vacant,
        None,
        Some(cardinal_sides()),
        Some(cardinal_facing()),
    );
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
    spawn_emplacement(
        &mut app,
        6,
        5,
        EmplacementState::Vacant,
        None,
        Some(cardinal_sides()),
        Some(cardinal_facing()),
    );

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
    let emplacement = spawn_emplacement(
        &mut app,
        6,
        5,
        EmplacementState::Vacant,
        None,
        Some(cardinal_sides()),
        Some(cardinal_facing()),
    );

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
        .get::<MountedBy>(emplacement)
        .map(Relationship::get);
    let selected = app
        .world()
        .get_resource::<SelectedShooter>()
        .and_then(|selection| **selection);
    assert_eq!(
        occupant, selected,
        "the ganger the emplacement records as mounting it is the acting selection",
    );
}
