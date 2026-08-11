//! Reloading over a real socket: the sim's own outcome reaches the caller.

use bevy::ecs::entity::Entity;
use gdtf_app::qa_wire::{
    act::ActReply,
    deed::{ActDeedKindNet, ReloadOutcomeNet},
    refusal::ReloadRefusalNet,
};
use gdtf_qa_protocol::{command::RunOptions, message::QaRequest};

use super::{
    act_support::{
        LogBody, accepted, answered, battle_app_prepared, decode, next, selected, window,
    },
    battle_reads::{a_player_ganger, ganger_argument, token_of},
    command_exchange::{
        ACT_RELOAD, ACT_SELECT, LOG_READ, assert_refused_off_the_battle_screen, exchange_expected,
        run,
    },
    magazine_support::{empty_the_magazine, fill_the_magazine},
    socket_support::{TestResult, game_app_listening},
};

/// Why the fixture cannot host a reload case.
const NO_GANGER: &str = "the fixture must hold a player ganger with a ranged magazine to reload";

/// The three requests a reload case sends, in order.
fn reload_plan(ganger: Entity) -> Vec<QaRequest> {
    vec![
        run(ACT_SELECT, &ganger_argument(ganger), RunOptions::default()),
        run(ACT_RELOAD, "()", RunOptions::default()),
        run(LOG_READ, "(cap:Some(200))", RunOptions::default()),
    ]
}

#[test]
fn a_reload_that_refills_answers_the_window_it_opened() -> TestResult {
    let (replies, ganger) = exchange_expected(
        battle_app_prepared(
            |app| {
                let (ganger, _) = a_player_ganger(app)?;
                let rounds = empty_the_magazine(app, ganger)?;
                (*rounds == 0).then_some(ganger)
            },
            NO_GANGER,
        ),
        |ganger| reload_plan(*ganger),
    )?;
    let mut replies = replies.into_iter();
    let shooter = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let reloaded = accepted(ACT_RELOAD, next(ACT_RELOAD, &mut replies)?)?;
    let log = decode::<LogBody>(LOG_READ, next(LOG_READ, &mut replies)?)?;

    assert_eq!(
        shooter,
        Some(token_of(ganger)),
        "the reload goes to the selected shooter, so selection must have taken first",
    );
    let Some(window) = window(reloaded) else {
        unreachable!("a reload the sim made carries its window");
    };
    let deeds = log.deeds_by(token_of(ganger), window);
    assert!(
        deeds.contains(&ActDeedKindNet::Reloaded {
            outcome: ReloadOutcomeNet::Reloaded,
        }),
        "the sim records the refill it made, so the window this call opened must hold one: \
         {window:?} logged {deeds:?} in {:?}",
        log.entries,
    );
    Ok(())
}

#[test]
fn a_reload_on_a_full_magazine_answers_the_already_full_reason() -> TestResult {
    let (replies, ganger) = exchange_expected(
        battle_app_prepared(
            |app| {
                let (ganger, _) = a_player_ganger(app)?;
                let rounds = fill_the_magazine(app, ganger)?;
                (*rounds > 0).then_some(ganger)
            },
            NO_GANGER,
        ),
        |ganger| reload_plan(*ganger),
    )?;
    let mut replies = replies.into_iter();
    let _shooter = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let declined = answered(ACT_RELOAD, next(ACT_RELOAD, &mut replies)?)?;
    let log = decode::<LogBody>(LOG_READ, next(LOG_READ, &mut replies)?)?;

    assert_eq!(
        declined,
        ActReply::ReloadRefused {
            reason: ReloadRefusalNet::AlreadyFull,
        },
        "the sim declines a reload on a full magazine and still logs it, so the reply has to name \
         that reason rather than the window a refill answers: {:?}",
        log.entries,
    );
    let deeds = log.deeds_from(token_of(ganger));
    assert!(
        deeds.contains(&ActDeedKindNet::Reloaded {
            outcome: ReloadOutcomeNet::AlreadyFull,
        }),
        "the log carries the declined outcome too, so a client reading the window sees the same \
         answer the reply gave: {deeds:?} in {:?}",
        log.entries,
    );
    Ok(())
}

#[test]
fn the_reload_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, ACT_RELOAD, "()")?;
    Ok(())
}
