//! `act.open_door` over a real socket: it fires what the panel offers, and refuses without one.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::{act::ActRefusalNet, offer::OfferTargetNet, token::DoorToken};
use gdtf_battle_sim::{openable::OpenState, prelude::CellLevel};
use gdtf_qa_protocol::{command::RunOptions, ports::NetQaPort};
use gdtf_test_utils::advance_until;

use super::{
    super::{
        act_support::{caught_up, next},
        command_exchange::{
            ACT_OPEN_DOOR, assert_refused_off_the_battle_screen, exchange_inspecting, run,
        },
        socket_support::{TestError, TestResult, battle_app_listening, game_app_listening},
    },
    scene::{
        Accepted, SETTLE_BUDGET, a_neighbour, accepted, clear_doors_around, door_state, refused,
        select_a_player_ganger, settle, spawn_closed_door,
    },
};

/// A live battle whose selected shooter has exactly one closed door beside it: ours.
pub(super) fn door_beside_the_shooter() -> Result<(App, NetQaPort, Entity), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let (_shooter, at) = select_a_player_ganger(&mut app)?;
    clear_doors_around(&mut app, at)?;
    let beside = a_neighbour(&app, at)?;
    let door = spawn_closed_door(&mut app, beside);
    settle(&mut app);
    Ok((app, port, door))
}

/// A live battle whose selected shooter has no closed door beside it at all.
fn no_door_beside_the_shooter() -> Result<(App, NetQaPort, CellLevel), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let (_shooter, at) = select_a_player_ganger(&mut app)?;
    clear_doors_around(&mut app, at)?;
    settle(&mut app);
    Ok((app, port, at))
}

fn open_the_offered_door<T>(
    fixture: impl FnOnce() -> Result<(App, NetQaPort, T), TestError>,
) -> Result<(App, Accepted, T), TestError> {
    let (app, replies, carried) = exchange_inspecting(fixture, |_carried| {
        vec![caught_up(), run(ACT_OPEN_DOOR, "()", RunOptions::default())]
    })?;
    let mut replies = replies.into_iter();
    let _caught = next("wait", &mut replies)?;
    let opened = accepted(ACT_OPEN_DOOR, next(ACT_OPEN_DOOR, &mut replies)?)?;
    Ok((app, opened, carried))
}

#[test]
fn opening_the_offered_door_names_it_and_leaves_it_open_in_the_world() -> TestResult {
    let (mut app, opened, door) = open_the_offered_door(door_beside_the_shooter)?;

    assert_eq!(
        opened.target,
        OfferTargetNet::Door(DoorToken::new(door.to_bits())),
        "the reply names the door the panel was offering, which is the one the call fired at",
    );
    let toggled = advance_until(
        &mut app,
        |app| door_state(app, door) == Some(OpenState::Open),
        SETTLE_BUDGET,
    );
    assert!(
        toggled,
        "the sim's own toggle must leave the carried door Open; the act log holds no deed for a \
         door, so the world is the evidence — last state was {:?}",
        door_state(&app, door),
    );
    Ok(())
}

#[test]
fn open_door_refuses_with_no_offer_until_a_door_stands_beside_the_shooter() -> TestResult {
    let (_app, replies, _at) = exchange_inspecting(no_door_beside_the_shooter, |_at| {
        vec![caught_up(), run(ACT_OPEN_DOOR, "()", RunOptions::default())]
    })?;
    let mut replies = replies.into_iter();
    let _caught = next("wait", &mut replies)?;
    let reason = refused(ACT_OPEN_DOOR, next(ACT_OPEN_DOOR, &mut replies)?)?;
    assert_eq!(
        reason,
        ActRefusalNet::NoOffer,
        "with nothing on offer the command answers the one refusal it has, rather than \
         re-deciding legality the sim owns",
    );

    let (_app, opened, door) = open_the_offered_door(door_beside_the_shooter)?;
    assert_eq!(
        opened.target,
        OfferTargetNet::Door(DoorToken::new(door.to_bits())),
        "the same call accepts once a closed door stands beside the shooter, which is what \
         makes the refusal above mean something",
    );
    Ok(())
}

#[test]
fn open_door_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, ACT_OPEN_DOOR, "()")?;
    Ok(())
}
