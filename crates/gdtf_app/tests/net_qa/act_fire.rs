//! One shot over a real socket, aimed at a cell an enemy is standing on.

use bevy::ecs::entity::Entity;
use gdtf_app::qa_wire::{cell::CellLevelNet, deed::ActDeedKindNet};
use gdtf_qa_protocol::command::RunOptions;

use super::{
    act_support::{
        LogBody, accepted, battle_app_reporting, complete, decode, next, selected, window,
    },
    battle_reads::{
        a_player_ganger, an_enemy_ganger_at, an_unreachable_cell, cell_argument, ganger_argument,
        token_of,
    },
    command_exchange::{
        ACT_FIRE, ACT_SELECT, LOG_READ, assert_refused_off_the_battle_screen, exchange_expected,
        run,
    },
    socket_support::{TestResult, game_app_listening},
};

/// Deed kinds a declared shot writes into the act log.
const SHOT_DEEDS: [ActDeedKindNet; 2] = [ActDeedKindNet::Fired, ActDeedKindNet::RoundResolved];

/// Why the fixture cannot host this case.
const NO_PAIR: &str = "the fixture must hold a player ganger and a living enemy to shoot at";

/// Why the fixture cannot host a case that only needs someone to pull the trigger.
const NO_SHOOTER: &str = "the fixture must hold a player ganger to shoot with";

/// The shooter this case fires with and the cell it fires at.
struct Shot {
    shooter: Entity,
    target:  CellLevelNet,
}

#[test]
fn one_shot_at_an_enemy_is_declared_in_the_window_it_opened() -> TestResult {
    let (replies, shot) = exchange_expected(
        battle_app_reporting(
            |app| {
                let (shooter, _) = a_player_ganger(app)?;
                let (_, target) = an_enemy_ganger_at(app)?;
                Some(Shot { shooter, target })
            },
            NO_PAIR,
        ),
        |shot| {
            vec![
                run(
                    ACT_SELECT,
                    &ganger_argument(shot.shooter),
                    RunOptions::default(),
                ),
                run(ACT_FIRE, &cell_argument(shot.target), RunOptions::default()),
                run(LOG_READ, "()", RunOptions::default()),
            ]
        },
    )?;
    let mut replies = replies.into_iter();
    let shooter = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let fired = accepted(ACT_FIRE, next(ACT_FIRE, &mut replies)?)?;
    let log = decode::<LogBody>(LOG_READ, next(LOG_READ, &mut replies)?)?;

    assert_eq!(
        shooter,
        Some(token_of(shot.shooter)),
        "the shot comes from the ganger the token named, so selection must have taken",
    );
    let Some(window) = window(fired) else {
        unreachable!("an accepted shot carries its window");
    };
    let deeds = log.deeds_by(token_of(shot.shooter), window);
    assert!(
        deeds.iter().any(|kind| SHOT_DEEDS.contains(kind)),
        "the sim must have declared the shot inside the window this call opened: {window:?} \
         logged {deeds:?} in {:?}",
        log.entries,
    );
    assert_eq!(
        complete(fired).map(|done| *done),
        Some(true),
        "a shot resolves inside its own frame, so it never reports itself as still playing out",
    );
    Ok(())
}

#[test]
fn a_shot_the_sim_declines_is_never_declared() -> TestResult {
    let (replies, shot) = exchange_expected(
        battle_app_reporting(
            |app| {
                let (shooter, _) = a_player_ganger(app)?;
                Some(Shot {
                    shooter,
                    target: an_unreachable_cell(),
                })
            },
            NO_SHOOTER,
        ),
        |shot| {
            vec![
                run(
                    ACT_SELECT,
                    &ganger_argument(shot.shooter),
                    RunOptions::default(),
                ),
                run(ACT_FIRE, &cell_argument(shot.target), RunOptions::default()),
                run(LOG_READ, "(cap:Some(200))", RunOptions::default()),
            ]
        },
    )?;
    let mut replies = replies.into_iter();
    let _selected = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let declined = accepted(ACT_FIRE, next(ACT_FIRE, &mut replies)?)?;
    let log = decode::<LogBody>(LOG_READ, next(LOG_READ, &mut replies)?)?;

    let Some(window) = window(declined) else {
        unreachable!("a declined shot is still an accepted call");
    };
    let (from, to) = window;
    assert_eq!(
        from, to,
        "a call the sim declined logs nothing at all, so its window closes where it opened",
    );
    let deeds = log.deeds_by(token_of(shot.shooter), window);
    assert!(
        deeds.is_empty(),
        "a shot the sim declines is never declared, so the shooter logs nothing in the window \
         the call opened: {window:?} logged {deeds:?} in {:?}",
        log.entries,
    );
    assert_eq!(
        complete(declined).map(|done| *done),
        Some(true),
        "a shot that was never declared leaves nothing playing out",
    );
    Ok(())
}

#[test]
fn the_shot_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(
        game_app_listening,
        ACT_FIRE,
        &cell_argument(an_unreachable_cell()),
    )?;
    Ok(())
}
