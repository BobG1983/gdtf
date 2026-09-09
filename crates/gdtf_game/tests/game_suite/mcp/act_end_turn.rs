//! Ending the turn: it runs over a real socket, and it parks until the turn comes back.

use std::sync::mpsc::TryRecvError;

use bevy::app::App;
use cobalt_mcp_protocol::{
    command::{CommandAvailability, CommandName, CommandOutcome, CommandTiming, RunOptions},
    message::{McpRequest, McpResponse},
};
use gdtf_battle_sim::{battle::PlayerFaction, turn::ActiveFaction};
use gdtf_game::qa_wire::deed::ActDeedKindNet;

use super::{
    act_support::{LogBody, accepted, complete, decode, next, window},
    command_exchange::{
        ACT_END_TURN, LOG_READ, assert_refused_off_the_battle_screen, exchange, exchange_all, run,
    },
    socket_support::{TestResult, battle_app_listening, game_app_listening},
};
use crate::mcp::battle_fixture::{drive_into_battle_running, menu_app_with_mcp, run_request, send};

fn the_players_turn(app: &App) -> bool {
    let world = app.world();
    let (Some(active), Some(player)) = (
        world.get_resource::<ActiveFaction>().map(|active| **active),
        world.get_resource::<PlayerFaction>().map(|player| **player),
    ) else {
        return false;
    };
    active == player
}

#[test]
fn ending_the_turn_is_admitted_by_a_running_battle_over_the_socket() -> TestResult {
    let reply = exchange(battle_app_listening, McpRequest::Catalogue)?;
    let McpResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    let Some(entry) = catalogue
        .entries
        .iter()
        .find(|entry| entry.command == CommandName::from_static(ACT_END_TURN))
    else {
        unreachable!("the catalogue carries a row for {ACT_END_TURN}: {catalogue:?}");
    };
    assert_eq!(
        entry.availability,
        CommandAvailability::Available,
        "the live availability column is what the router admits on, so a running caught-up \
         battle must read as available rather than unavailable: {entry:?}",
    );
    assert_eq!(
        entry.timing,
        CommandTiming::Deferred,
        "the reply is held until the turn returns, so the row must say Deferred: {entry:?}",
    );
    Ok(())
}

#[test]
fn ending_the_turn_over_the_socket_answers_across_the_enemy_turn() -> TestResult {
    let replies = exchange_all(
        battle_app_listening,
        vec![
            run(ACT_END_TURN, "()", RunOptions::default()),
            run(LOG_READ, "(cap:Some(200))", RunOptions::default()),
        ],
    )?;
    let mut replies = replies.into_iter();
    let ended = accepted(ACT_END_TURN, next(ACT_END_TURN, &mut replies)?)?;
    let log = decode::<LogBody>(LOG_READ, next(LOG_READ, &mut replies)?)?;

    let Some((from, to)) = window(ended) else {
        unreachable!("an accepted end turn carries its window");
    };
    let handovers = log
        .entries
        .iter()
        .filter(|entry| entry.seq >= from && entry.seq < to)
        .filter(|entry| entry.kind == ActDeedKindNet::TurnBegan)
        .count();
    assert!(
        handovers >= 2,
        "the reply is held until the turn is back with the player, so its window brackets both \
         the hand-over to the enemy and the hand-back: {from:?}..{to:?} held {handovers} in {:?}",
        log.entries,
    );
    assert_eq!(
        complete(ended).map(|done| *done),
        Some(true),
        "`act.end_turn` has no actor, so nobody can still be walking an act out and this flag \
         is always true — anything that makes it conditional has to move the docs on \
         `ActReply::Accepted`, the command summary and the QA guide with it",
    );
    Ok(())
}

#[test]
fn ending_the_turn_parks_until_the_turn_comes_back_to_the_player() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);
    assert!(
        the_players_turn(&app),
        "this case starts on the player's turn, or parking proves nothing",
    );

    let reply = send(&tx, run_request(ACT_END_TURN, "()"));
    let mut handed_over = false;
    let mut frame = 0_u32;
    let answered = loop {
        app.update();
        frame += 1;
        let players_turn = the_players_turn(&app);
        if !players_turn {
            handed_over = true;
        }
        match reply.try_recv() {
            Ok(landed) => {
                assert!(
                    handed_over,
                    "the reply must not land before the turn has left the player at all — it \
                     answered on frame {frame}",
                );
                assert!(
                    players_turn,
                    "the reply lands only once the turn is back with the player — it answered \
                     on frame {frame} while the enemy was acting",
                );
                break landed;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                unreachable!("the responder outlives the parked call")
            }
        }
    };

    assert!(
        handed_over,
        "ending the turn must actually hand it to the enemy, or nothing was being waited on",
    );
    assert!(
        matches!(answered, McpResponse::Outcome(CommandOutcome::Ran { .. })),
        "the enemy AI ends its own turn once it can neither shoot nor step, which hands the turn \
         back and releases the parked call; got {answered:?}",
    );
}

#[test]
fn ending_the_turn_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, ACT_END_TURN, "()")?;
    Ok(())
}
