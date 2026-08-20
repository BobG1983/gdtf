//! What each of the two log reads tells a caller about a gang the squad cannot see.

use gdtf_app::qa_wire::{deed::ActDeedKindNet, log::LogEntryNet, token::GangerToken};
use gdtf_qa_protocol::{
    command::RunOptions,
    message::{QaRequest, QaResponse},
};
use serde::Deserialize;

use super::{
    battle_setup::battle_with_a_hidden_enemy_shooting_across_the_lit_area,
    command_exchange::{
        LOG_OMNISCIENT_READ, LOG_READ, assert_refused_off_the_battle_screen, exchange_expected,
        ran_body, run,
    },
    socket_support::{TestError, TestResult, game_app_listening},
};

/// Cap wide enough that both of the fixture's acts sit inside one window.
const WIDE_CAP: u32 = 200;

#[derive(Debug, Deserialize)]
struct LogBody {
    entries: Vec<LogEntryNet>,
}

fn wide_read(command: &'static str) -> QaRequest {
    run(
        command,
        &format!("(cap:Some({WIDE_CAP}))"),
        RunOptions::default(),
    )
}

fn decode_as(command: &'static str, reply: Option<QaResponse>) -> Result<LogBody, TestError> {
    let Some(reply) = reply else {
        return Err(format!("{command} produced no reply").into());
    };
    let body = ran_body(command, reply)?;
    ron::de::from_str::<LogBody>(&body)
        .map_err(|fault| format!("the log body must decode: {fault} — {body}").into())
}

/// Read both log commands over one battle whose enemy fired from the dark across the light.
fn gated_and_omniscient() -> Result<(LogBody, LogBody, GangerToken), TestError> {
    let (replies, shooter) = exchange_expected(
        battle_with_a_hidden_enemy_shooting_across_the_lit_area,
        |_shooter| vec![wide_read(LOG_READ), wide_read(LOG_OMNISCIENT_READ)],
    )?;
    let mut replies = replies.into_iter();
    let gated = decode_as(LOG_READ, replies.next())?;
    let omniscient = decode_as(LOG_OMNISCIENT_READ, replies.next())?;
    Ok((
        gated,
        omniscient,
        GangerToken::new(shooter.entity.to_bits()),
    ))
}

#[test]
fn the_read_withholds_an_act_the_squad_could_not_observe() -> TestResult {
    let (gated, omniscient, token) = gated_and_omniscient()?;

    let withheld: Vec<&LogEntryNet> = gated
        .entries
        .iter()
        .filter(|entry| entry.kind == ActDeedKindNet::Suppressed)
        .collect();
    assert!(
        withheld.is_empty(),
        "the enemy was suppressed on its own unlit cell, so `log.read` must return no \
         {kind:?} line at all — it leaked {withheld:?}",
        kind = ActDeedKindNet::Suppressed,
    );
    assert!(
        omniscient
            .entries
            .iter()
            .any(|entry| entry.kind == ActDeedKindNet::Suppressed && entry.actor == Some(token)),
        "`log.omniscient_read` returns the same line, naming the enemy: {:?}",
        omniscient.entries,
    );
    Ok(())
}

#[test]
fn the_read_reports_a_shot_across_the_lit_area_without_naming_its_shooter() -> TestResult {
    let (gated, omniscient, token) = gated_and_omniscient()?;

    let fired: Vec<&LogEntryNet> = gated
        .entries
        .iter()
        .filter(|entry| entry.kind == ActDeedKindNet::Fired)
        .collect();
    assert_eq!(
        fired.len(),
        1,
        "the shot's flight crossed a cell the squad watches, so the line is reported: {:?}",
        gated.entries,
    );
    for entry in fired {
        assert_eq!(
            entry.actor, None,
            "the shooter stood in the dark, so its token {token:?} must not reach the \
             caller: {entry:?}",
        );
    }
    assert!(
        omniscient
            .entries
            .iter()
            .any(|entry| entry.kind == ActDeedKindNet::Fired && entry.actor == Some(token)),
        "`log.omniscient_read` returns the same shot with the enemy's token on it: {:?}",
        omniscient.entries,
    );
    Ok(())
}

#[test]
fn the_omniscient_read_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, LOG_OMNISCIENT_READ, "()")?;
    Ok(())
}
