//! Two requests for the same emplacement act in one frame: the leaf is charged once.

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::{EnterEmplacementRequested, ExitEmplacementRequested},
    ganger::Direction,
    metric::CellLevel,
    terrain::emplacement::EmplacementState,
    test_support::{SituationBuilder, emplacement_at},
};

use super::harness::*;

/// The seed both runs drive, so they differ only in the messages written into the frame.
const SEED: u64 = 0x5543_1238;

/// The exit leaf every run tunes: positive, and not the harness default of 4, so a run reading a
/// stale leaf is visible in the failure message.
const EXIT_TU: u8 = 11;

/// Ticks a toggle is given to settle: the frame that dispatches it, and the ones that apply it.
const SETTLE_TICKS: u32 = 3;

/// The cell the actor starts on, beside the seat.
fn start() -> CellLevel {
    ground(5, 5)
}

/// A battle with one player actor on [`start`] and one vacant all-sided emplacement on [`seat`].
fn a_seat_beside_the_actor() -> (App, Entity, Entity) {
    let (mut app, seed) = battle_app(SEED);
    set_exit_tu(&mut app, EXIT_TU);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(start(), Direction::East)])
        .with_scatter(emplacement_at(seat()))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, seat());
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
    (app, actor, emplacement)
}

/// TU the actor has lost since `before`.
fn tu_lost(app: &App, actor: Entity, before: u8) -> u8 {
    before.saturating_sub(tu_of(app, actor).unwrap_or(0))
}

#[test]
fn two_exit_requests_in_one_frame_charge_the_exit_leaf_once() {
    let (mut app, actor, emplacement) = a_seat_beside_the_actor();
    let leaf = exit_tu(&app);
    assert_eq!(
        leaf, EXIT_TU,
        "PRECONDITION: the run must hold the exit leaf it tuned, it reads {leaf}",
    );
    assert!(
        leaf > 0,
        "PRECONDITION: the exit leaf must be positive, or the charge assertion below holds \
         whatever the dispatcher does; it reads {leaf}",
    );
    mount(&mut app, actor, emplacement);

    let before = tu_of(&app, actor).unwrap_or(0);
    for _ in 0..2 {
        app.world_mut()
            .write_message(ExitEmplacementRequested::new(actor, emplacement));
    }
    step(&mut app, SETTLE_TICKS);

    let lost = tu_lost(&app, actor, before);
    assert_eq!(
        lost, leaf,
        "two exit requests in one frame must charge the exit leaf once: the actor lost {lost} TU \
         against a leaf of {leaf}",
    );
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "the one charge must buy the one dismount, so the seat ends Vacant against a leaf of \
         {leaf}; it reads {:?}",
        state(&app, emplacement),
    );
    assert_eq!(
        occupant(&app, emplacement),
        None,
        "the vacated seat must name no occupant against a leaf of {leaf}; it names {:?}",
        occupant(&app, emplacement),
    );
}

#[test]
fn two_enter_requests_in_one_frame_charge_the_enter_leaf_once() {
    let (mut app, actor, emplacement) = a_seat_beside_the_actor();
    let leaf = enter_tu(&app);
    assert!(
        leaf > 0,
        "PRECONDITION: the enter leaf must be positive, or the charge assertion below holds \
         whatever the dispatcher does; it reads {leaf}",
    );
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "PRECONDITION: the seat must start Vacant, or the enters are refused and nothing is \
         charged; it reads {:?}",
        state(&app, emplacement),
    );

    let before = tu_of(&app, actor).unwrap_or(0);
    for _ in 0..2 {
        app.world_mut()
            .write_message(EnterEmplacementRequested::new(actor, emplacement));
    }
    step(&mut app, SETTLE_TICKS);

    let lost = tu_lost(&app, actor, before);
    assert_eq!(
        lost, leaf,
        "two enter requests in one frame must charge the enter leaf once: the actor lost {lost} \
         TU against a leaf of {leaf}",
    );
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "the one charge must buy the one mount, so the seat ends Occupied against a leaf of \
         {leaf}; it reads {:?}",
        state(&app, emplacement),
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(actor),
        "the manned seat must name the actor against a leaf of {leaf}; it names {:?}",
        occupant(&app, emplacement),
    );
}
