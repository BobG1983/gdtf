use gdtf_app::qa_wire::roster::FactionNet;
use gdtf_qa_protocol::{command::RunOptions, message::QaResponse};
use serde::Deserialize;

use super::{
    battle_setup::battle_with_another_gang_acting,
    command_exchange::{
        BATTLE_TURN, assert_refused_off_the_battle_screen, exchange_expected, ran_body, run,
    },
    socket_support::{TestError, TestResult, game_app_listening},
};

#[derive(Debug, Deserialize)]
struct TurnBody {
    active: Option<FactionNet>,
    player: Option<FactionNet>,
}

fn turn_body(reply: Option<QaResponse>) -> Result<TurnBody, TestError> {
    let Some(reply) = reply else {
        return Err("battle.turn produced no reply".into());
    };
    let body = ran_body(BATTLE_TURN, reply)?;
    ron::de::from_str::<TurnBody>(&body)
        .map_err(|fault| format!("the turn body must decode: {fault} — {body}").into())
}

#[test]
fn the_turn_names_the_gangs_the_sim_holds() -> TestResult {
    let (replies, live) = exchange_expected(battle_with_another_gang_acting, |_live| {
        vec![run(BATTLE_TURN, "()", RunOptions::default())]
    })?;
    let turn = turn_body(replies.into_iter().next())?;

    assert_ne!(
        live.active, live.player,
        "the fixture must really split the acting gang from the player's own, or this case \
         cannot tell the two fields apart: {live:?}",
    );
    assert_eq!(
        turn.active,
        Some(live.active),
        "the acting gang is the one the sim's turn cycle holds, not a fixed index: {turn:?} \
         against {live:?}",
    );
    assert_eq!(
        turn.player,
        Some(live.player),
        "the player's gang is the one the battle was set up with: {turn:?} against {live:?}",
    );
    Ok(())
}

#[test]
fn the_turn_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, BATTLE_TURN, "()")?;
    Ok(())
}
