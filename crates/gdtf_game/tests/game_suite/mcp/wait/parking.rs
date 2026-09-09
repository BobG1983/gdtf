use core::time::Duration;
use std::sync::mpsc::TryRecvError;

use cobalt_mcp_host::dispatch::DeferredBudget;
use cobalt_mcp_protocol::{
    command::CommandOutcome,
    message::{McpResponse, McpSessionError},
};
use gdtf_game::test_support::shorten_wait_budget;

use super::support::{PARKED_FRAMES, answered_within, holding_battle_with_mcp, release_the_hold};
use crate::mcp::{
    battle_fixture::{drive_into_battle_running, menu_app_with_mcp, run_request, send},
    command_exchange::WAIT,
};

#[test]
fn wait_holds_its_reply_until_the_condition_it_named_comes_true() {
    let (mut app, tx) = holding_battle_with_mcp();

    let reply = send(&tx, run_request(WAIT, "(condition:CaughtUp)"));
    for frame in 0..PARKED_FRAMES {
        app.update();
        assert_eq!(
            reply.try_recv().err(),
            Some(TryRecvError::Empty),
            "the cursor is still holding, so the screen has not caught up and `wait` must not \
             have answered — it answered on frame {frame}",
        );
    }

    release_the_hold(&mut app);

    let answered = answered_within(&mut app, &reply);
    let McpResponse::Outcome(CommandOutcome::Ran { reply, .. }) = answered else {
        unreachable!(
            "once the hold is gone and the cursor sits on the log head the gate is open, so the \
             parked `wait` must be released; got {answered:?}"
        );
    };
    assert!(
        reply.as_str().contains("CaughtUp"),
        "the reply names the condition that came true: {}",
        reply.as_str(),
    );
}

#[test]
fn wait_on_a_phase_parks_in_the_menu_and_settles_once_the_battle_is_up() {
    let (mut app, tx) = menu_app_with_mcp();
    let reply = send(
        &tx,
        run_request(WAIT, "(condition:Phase((game:Some(BattleScape))))"),
    );
    for frame in 0..PARKED_FRAMES {
        app.update();
        assert_eq!(
            reply.try_recv().err(),
            Some(TryRecvError::Empty),
            "the app is still in the Menu, so a wait for the battle layer must stay parked — it \
             answered on frame {frame}",
        );
    }

    drive_into_battle_running(&mut app);

    let answered = answered_within(&mut app, &reply);
    let McpResponse::Outcome(CommandOutcome::Ran { reply, .. }) = answered else {
        unreachable!(
            "once the game layer is the battle map the parked wait must be released; got \
             {answered:?}"
        );
    };
    assert!(
        reply.as_str().contains("game:Some(BattleScape)"),
        "the reply reports the phase the app landed in: {}",
        reply.as_str(),
    );
}

#[test]
fn a_condition_that_never_comes_true_times_out_rather_than_being_refused() {
    let (mut app, tx) = menu_app_with_mcp();
    shorten_wait_budget(&mut app, DeferredBudget::new(Duration::ZERO));

    let reply = send(&tx, run_request(WAIT, "(condition:TurnChanged)"));
    let answered = answered_within(&mut app, &reply);
    assert_eq!(
        answered,
        McpResponse::Error(McpSessionError::Timeout),
        "a condition nothing will ever make true must expire against the parking budget, not \
         come back as a refusal",
    );
}
