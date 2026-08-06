//! Selection over a real socket: cycle forward, back, clear, and pick by token.

use bevy::app::App;
use gdtf_app::qa_wire::token::GangerToken;
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{
    command::{CommandOutcome, RunOptions},
    message::QaResponse,
};

use super::{
    act_support::{SelectionBody, decode, next, selected},
    battle_reads::{an_enemy_ganger, an_unselected_player_ganger, player_gangers, token_of},
    command_exchange::{
        ACT_SELECT, ACT_SELECT_CLEAR, ACT_SELECT_NEXT, ACT_SELECT_PREV, BATTLE_SELECTION,
        assert_refused_off_the_battle_screen, exchange, exchange_all, exchange_expected, run,
    },
    socket_support::{TestError, TestResult, battle_app_listening, game_app_listening},
};

/// A token that cannot name a live ganger, whatever the world holds.
const NOT_A_GANGER: GangerToken = GangerToken::new(u64::MAX);

/// The first entity the process ever spawned, which is not one of the battle's gangers.
const NOT_A_GANGER_EITHER: GangerToken = GangerToken::new(0);

fn select_argument(token: GangerToken) -> String {
    format!("(ganger:{})", *token)
}

/// A running battle plus the token of a player ganger the game has not already selected.
///
/// The game re-selects the first living player ganger the moment nothing is selected, so a
/// case that picks that one cannot tell `act.select` from the game's own re-pick.
fn battle_naming_an_unselected_ganger() -> Result<(App, NetQaPort, GangerToken), TestError> {
    let (app, port) = battle_app_listening()?;
    let Some(entity) = an_unselected_player_ganger(&app) else {
        return Err(
            "a running battle must hold a player ganger other than the selected one".into(),
        );
    };
    Ok((app, port, token_of(entity)))
}

/// A running battle plus the token of a ganger the player does not command.
fn battle_naming_an_enemy() -> Result<(App, NetQaPort, GangerToken), TestError> {
    let (app, port) = battle_app_listening()?;
    let Some(entity) = an_enemy_ganger(&app) else {
        return Err("a running battle must hold a living enemy ganger".into());
    };
    Ok((app, port, token_of(entity)))
}

#[test]
fn the_selection_cycles_forward_back_and_clears() -> TestResult {
    let (replies, gangers) = exchange_expected(
        || {
            let (app, port) = battle_app_listening()?;
            let count = player_gangers(&app).len();
            Ok((app, port, count))
        },
        |_count| {
            vec![
                run(ACT_SELECT_NEXT, "()", RunOptions::default()),
                run(ACT_SELECT_NEXT, "()", RunOptions::default()),
                run(ACT_SELECT_PREV, "()", RunOptions::default()),
                run(ACT_SELECT_CLEAR, "()", RunOptions::default()),
                run(BATTLE_SELECTION, "()", RunOptions::default()),
            ]
        },
    )?;
    assert!(
        gangers >= 2,
        "this case needs two player gangers to tell a cycle from a no-op; the fixture spawned \
         {gangers}",
    );

    let mut replies = replies.into_iter();
    let first = selected(ACT_SELECT_NEXT, next(ACT_SELECT_NEXT, &mut replies)?)?;
    let second = selected(ACT_SELECT_NEXT, next(ACT_SELECT_NEXT, &mut replies)?)?;
    let back = selected(ACT_SELECT_PREV, next(ACT_SELECT_PREV, &mut replies)?)?;
    let cleared = selected(ACT_SELECT_CLEAR, next(ACT_SELECT_CLEAR, &mut replies)?)?;
    let after = decode::<SelectionBody>(BATTLE_SELECTION, next(BATTLE_SELECTION, &mut replies)?)?;

    assert!(
        first.is_some() && second.is_some(),
        "a running battle has player gangers to cycle through: {first:?} then {second:?}",
    );
    assert_ne!(
        first, second,
        "stepping forward twice must land on a different ganger, or the cycle did nothing",
    );
    assert_eq!(
        back, first,
        "stepping back from the second selection returns to the first — on this fixture's ring \
         that only says the step happened, so which way each command walks is pinned by \
         `stepping_back_retraces_the_way_the_cycle_came`: {back:?}",
    );
    assert_eq!(
        cleared, None,
        "the clear itself leaves nobody selected in the frame it lands: {cleared:?}",
    );
    assert!(
        after.shooter.is_some(),
        "the game hands the player someone to act with, so a cleared selection is filled again \
         on the next frame and every act after a clear runs through whoever was re-picked — the \
         empty reply is not a lasting state: {after:?}",
    );
    Ok(())
}

#[test]
fn selecting_a_ganger_by_token_lands_on_that_ganger() -> TestResult {
    let (replies, token) = exchange_expected(battle_naming_an_unselected_ganger, |token| {
        vec![
            run(ACT_SELECT_CLEAR, "()", RunOptions::default()),
            run(ACT_SELECT, &select_argument(*token), RunOptions::default()),
        ]
    })?;
    let mut replies = replies.into_iter();
    let cleared = selected(ACT_SELECT_CLEAR, next(ACT_SELECT_CLEAR, &mut replies)?)?;
    let chosen = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;

    assert_eq!(
        cleared, None,
        "the case clears first so the pick cannot be the selection that was already there",
    );
    assert_eq!(
        chosen,
        Some(token),
        "selecting a player ganger by token must land on that ganger, not on the one the game \
         re-picks for itself after a clear",
    );
    Ok(())
}

#[test]
fn a_token_for_an_enemy_leaves_the_selection_where_it_was() -> TestResult {
    let (replies, enemy) = exchange_expected(battle_naming_an_enemy, |enemy| {
        vec![
            run(ACT_SELECT_NEXT, "()", RunOptions::default()),
            run(ACT_SELECT, &select_argument(*enemy), RunOptions::default()),
        ]
    })?;
    let mut replies = replies.into_iter();
    let before = selected(ACT_SELECT_NEXT, next(ACT_SELECT_NEXT, &mut replies)?)?;
    let after = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;

    assert!(
        before.is_some(),
        "the case needs a selection to leave alone: {before:?}",
    );
    assert_ne!(
        after,
        Some(enemy),
        "the wire selects nobody the pointer would not, and the game only ever selects a ganger \
         the player commands: {after:?}",
    );
    assert_eq!(
        after, before,
        "a token the game will not take leaves the selection where it was: {after:?}",
    );
    Ok(())
}

#[test]
fn a_token_that_names_nothing_is_refused_rather_than_crashing() -> TestResult {
    for token in [NOT_A_GANGER, NOT_A_GANGER_EITHER] {
        let reply = exchange(
            battle_app_listening,
            run(ACT_SELECT, &select_argument(token), RunOptions::default()),
        )?;
        let QaResponse::Outcome(CommandOutcome::Ran { reply, .. }) = reply else {
            return Err(format!("`{ACT_SELECT}` must answer, got {reply:?}").into());
        };
        let body = reply.as_str();
        assert!(
            body.contains("Refused") && body.contains("UnknownToken"),
            "a token naming no living ganger must be refused UnknownToken, got {body}",
        );
    }
    Ok(())
}

#[test]
fn a_selection_command_answers_the_same_shape_every_time() -> TestResult {
    let replies = exchange_all(
        battle_app_listening,
        vec![run(ACT_SELECT_NEXT, "()", RunOptions::default())],
    )?;
    let mut replies = replies.into_iter();
    let shooter = selected(ACT_SELECT_NEXT, next(ACT_SELECT_NEXT, &mut replies)?)?;
    assert!(
        shooter.is_some(),
        "stepping the cycle in a running battle always lands on someone: {shooter:?}",
    );
    Ok(())
}

#[test]
fn every_selection_command_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, ACT_SELECT_NEXT, "()")?;
    assert_refused_off_the_battle_screen(game_app_listening, ACT_SELECT_PREV, "()")?;
    assert_refused_off_the_battle_screen(game_app_listening, ACT_SELECT_CLEAR, "()")?;
    assert_refused_off_the_battle_screen(
        game_app_listening,
        ACT_SELECT,
        &select_argument(NOT_A_GANGER),
    )?;
    Ok(())
}
