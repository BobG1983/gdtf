//! Moves over a real socket: the ganger arrives, a long walk reports itself unfinished, and a
//! refused move carries the sim's own reason.

use bevy::ecs::entity::Entity;
use gdtf_app::qa_wire::{
    cell::CellLevelNet,
    deed::{ActDeedKindNet, MoveRejectionNet},
};
use gdtf_qa_protocol::command::RunOptions;

use super::{
    act_support::{
        LogBody, RosterBody, accepted, battle_app_reporting, card_of, complete, decode, next,
        selected, walk_complete, window,
    },
    battle_reads::{
        a_player_ganger, an_unreachable_cell, cell_argument, ganger_argument, one_step_from,
        token_of, two_steps_from,
    },
    command_exchange::{
        ACT_MOVE, ACT_SELECT, BATTLE_ROSTER, LOG_READ, WAIT, assert_refused_off_the_battle_screen,
        exchange_expected, run,
    },
    socket_support::{TestResult, game_app_listening},
};

/// Deed kinds a walk writes into the act log.
const WALK_DEEDS: [ActDeedKindNet; 2] = [ActDeedKindNet::Stepped, ActDeedKindNet::MovedTo];

/// Why the fixture cannot host a move case.
const NO_GANGER: &str = "the fixture must hold a player ganger to plan a move from";

/// Why the fixture cannot host the arrival case.
const NO_STEP: &str = "the generated map must offer one clear step from the ganger";

/// Why the fixture cannot host the long-walk case.
const NO_ROOM: &str = "the generated map must offer two clear steps in a line from the ganger";

/// The ganger a move case walks, and where it is sent.
struct Walk {
    ganger: Entity,
    to:     CellLevelNet,
}

#[test]
fn one_move_walks_the_selected_ganger_onto_the_cell_it_named() -> TestResult {
    let (replies, walk) = exchange_expected(
        battle_app_reporting(
            |app| {
                let (ganger, at) = a_player_ganger(app)?;
                Some(Walk {
                    ganger,
                    to: one_step_from(app, at)?,
                })
            },
            NO_STEP,
        ),
        |walk| {
            vec![
                run(
                    ACT_SELECT,
                    &ganger_argument(walk.ganger),
                    RunOptions::default(),
                ),
                run(ACT_MOVE, &cell_argument(walk.to), RunOptions::default()),
                walk_complete(),
                run(LOG_READ, "(cap:Some(200))", RunOptions::default()),
                run(BATTLE_ROSTER, "()", RunOptions::default()),
            ]
        },
    )?;
    let mut replies = replies.into_iter();
    let shooter = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let moved = accepted(ACT_MOVE, next(ACT_MOVE, &mut replies)?)?;
    let _walked = next(WAIT, &mut replies)?;
    let log = decode::<LogBody>(LOG_READ, next(LOG_READ, &mut replies)?)?;
    let roster = decode::<RosterBody>(BATTLE_ROSTER, next(BATTLE_ROSTER, &mut replies)?)?;

    let actor = token_of(walk.ganger);
    assert_eq!(
        shooter,
        Some(actor),
        "the move goes to the selected shooter, so selection must have taken first",
    );
    let Some(window) = window(moved) else {
        unreachable!("an accepted move carries its window");
    };
    let deeds = log.deeds_by(actor, window);
    assert!(
        deeds.iter().any(|kind| WALK_DEEDS.contains(kind)),
        "the sim must have logged this ganger's walk inside the window this move opened: \
         {window:?} logged {deeds:?} in {:?}",
        log.entries,
    );
    let Some(card) = card_of(&roster, actor) else {
        unreachable!("the ganger that walked is on the roster: {roster:?}");
    };
    assert_eq!(
        card.at, walk.to,
        "the walk ends on the cell the call named, not merely somewhere: {card:?}",
    );
    Ok(())
}

#[test]
fn a_move_still_walking_its_route_reports_itself_incomplete() -> TestResult {
    let (replies, walk) = exchange_expected(
        battle_app_reporting(
            |app| {
                let (ganger, at) = a_player_ganger(app)?;
                Some(Walk {
                    ganger,
                    to: two_steps_from(app, at)?,
                })
            },
            NO_ROOM,
        ),
        |walk| {
            vec![
                run(
                    ACT_SELECT,
                    &ganger_argument(walk.ganger),
                    RunOptions::default(),
                ),
                run(ACT_MOVE, &cell_argument(walk.to), RunOptions::default()),
                run(LOG_READ, "(cap:Some(200))", RunOptions::default()),
            ]
        },
    )?;
    let mut replies = replies.into_iter();
    let _shooter = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let moved = accepted(ACT_MOVE, next(ACT_MOVE, &mut replies)?)?;
    let log = decode::<LogBody>(LOG_READ, next(LOG_READ, &mut replies)?)?;

    let actor = token_of(walk.ganger);
    let Some(window) = window(moved) else {
        unreachable!("an accepted move carries its window");
    };
    let deeds = log.deeds_by(actor, window);
    assert!(
        deeds.iter().any(|kind| WALK_DEEDS.contains(kind)),
        "a two-step route has to start walking in the frame it was claimed: {window:?} logged \
         {deeds:?} in {:?}",
        log.entries,
    );
    assert_eq!(
        complete(moved).map(|done| *done),
        Some(false),
        "the sim walks one cell per frame, so a two-step route is still being walked when the \
         reply goes out — a posture or a shot reports itself finished instead",
    );
    Ok(())
}

#[test]
fn a_move_the_sim_refuses_carries_the_sim_reason_into_the_log() -> TestResult {
    let (replies, walk) = exchange_expected(
        battle_app_reporting(
            |app| {
                let (ganger, _) = a_player_ganger(app)?;
                Some(Walk {
                    ganger,
                    to: an_unreachable_cell(),
                })
            },
            NO_GANGER,
        ),
        |walk| {
            vec![
                run(
                    ACT_SELECT,
                    &ganger_argument(walk.ganger),
                    RunOptions::default(),
                ),
                run(ACT_MOVE, &cell_argument(walk.to), RunOptions::default()),
                run(LOG_READ, "(cap:Some(200))", RunOptions::default()),
            ]
        },
    )?;
    let mut replies = replies.into_iter();
    let _shooter = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let refused = accepted(ACT_MOVE, next(ACT_MOVE, &mut replies)?)?;
    let log = decode::<LogBody>(LOG_READ, next(LOG_READ, &mut replies)?)?;

    let actor = token_of(walk.ganger);
    let Some(window) = window(refused) else {
        unreachable!("a move the sim turns down is still an accepted call");
    };
    let deeds = log.deeds_by(actor, window);
    assert!(
        deeds.contains(&ActDeedKindNet::MoveRefused {
            reason: MoveRejectionNet::Unreachable,
        }),
        "the QA layer invents no refusal of its own: the sim's reason for turning a move down is \
         read out of the window the call opened: {window:?} logged {deeds:?} in {:?}",
        log.entries,
    );
    Ok(())
}

#[test]
fn the_move_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(
        game_app_listening,
        ACT_MOVE,
        &cell_argument(an_unreachable_cell()),
    )?;
    Ok(())
}
