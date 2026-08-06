//! Reloading over a real socket: the sim records the reload the command asked for.

use gdtf_app::qa_wire::deed::ActDeedKindNet;
use gdtf_qa_protocol::command::RunOptions;

use super::{
    act_support::{LogBody, accepted, battle_app_reporting, decode, next, selected, window},
    battle_reads::{a_player_ganger, ganger_argument, token_of},
    command_exchange::{
        ACT_RELOAD, ACT_SELECT, LOG_READ, assert_refused_off_the_battle_screen, exchange_expected,
        run,
    },
    socket_support::{TestResult, game_app_listening},
};

/// Why the fixture cannot host this case.
const NO_GANGER: &str = "the fixture must hold a player ganger to reload with";

#[test]
fn a_reload_is_recorded_in_the_window_it_opened() -> TestResult {
    let (replies, ganger) = exchange_expected(
        battle_app_reporting(
            |app| a_player_ganger(app).map(|(ganger, _)| ganger),
            NO_GANGER,
        ),
        |ganger| {
            vec![
                run(ACT_SELECT, &ganger_argument(*ganger), RunOptions::default()),
                run(ACT_RELOAD, "()", RunOptions::default()),
                run(LOG_READ, "(cap:Some(200))", RunOptions::default()),
            ]
        },
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
        unreachable!("an accepted reload carries its window");
    };
    let deeds = log.deeds_by(token_of(ganger), window);
    assert!(
        deeds.contains(&ActDeedKindNet::Reloaded),
        "the sim records every reload it hears, whatever the outcome, so the window this call \
         opened must hold one: {window:?} logged {deeds:?} in {:?}",
        log.entries,
    );
    Ok(())
}

#[test]
fn the_reload_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, ACT_RELOAD, "()")?;
    Ok(())
}
