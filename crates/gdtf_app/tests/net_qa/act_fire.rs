//! One shot over a real socket, aimed at a cell an enemy is standing on.

use bevy::ecs::entity::Entity;
use gdtf_app::qa_wire::{
    act::ActReply, cell::CellLevelNet, deed::ActDeedKindNet, refusal::ShotRefusalNet,
};
use gdtf_qa_protocol::command::RunOptions;

use super::{
    act_support::{
        LogBody, accepted, answered, battle_app_prepared, battle_app_reporting, complete, decode,
        next, selected, window,
    },
    battle_reads::{
        a_player_ganger, an_enemy_ganger_at, an_unreachable_cell, cell_argument, ganger_argument,
        token_of,
    },
    command_exchange::{
        ACT_FIRE, ACT_SELECT, LOG_READ, assert_refused_off_the_battle_screen, exchange_expected,
        run,
    },
    magazine_support::empty_the_magazine,
    socket_support::{TestResult, game_app_listening},
};

/// Deed kinds a declared shot writes into the act log.
const SHOT_DEEDS: [ActDeedKindNet; 2] = [ActDeedKindNet::Fired, ActDeedKindNet::RoundResolved];

/// Why the fixture cannot host this case.
const NO_PAIR: &str = "the fixture must hold a player ganger and a living enemy to shoot at";

/// Why the fixture cannot host a case that only needs someone to pull the trigger.
const NO_SHOOTER: &str = "the fixture must hold a player ganger to shoot with";

/// Why the fixture cannot host the dry-gun case.
const NO_MAGAZINE: &str =
    "the fixture must hold a player ganger with a ranged magazine and an enemy to shoot at";

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
fn a_shot_at_a_cell_off_the_grid_answers_the_out_of_bounds_reason() -> TestResult {
    let (replies, _shot) = exchange_expected(
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
    let declined = answered(ACT_FIRE, next(ACT_FIRE, &mut replies)?)?;
    let log = decode::<LogBody>(LOG_READ, next(LOG_READ, &mut replies)?)?;

    assert_eq!(
        declined,
        ActReply::FireRefused {
            reason: ShotRefusalNet::OutOfBounds,
        },
        "a target off the battle grid is a shot the game turns down, and the reply must name that \
         reason rather than an accepted window: {:?}",
        log.entries,
    );
    Ok(())
}

#[test]
fn a_shot_on_an_empty_magazine_answers_the_empty_magazine_reason() -> TestResult {
    let (replies, shot) = exchange_expected(
        battle_app_prepared(
            |app| {
                let (shooter, _) = a_player_ganger(app)?;
                let (_, target) = an_enemy_ganger_at(app)?;
                let rounds = empty_the_magazine(app, shooter)?;
                (*rounds == 0).then_some(Shot { shooter, target })
            },
            NO_MAGAZINE,
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
    let declined = answered(ACT_FIRE, next(ACT_FIRE, &mut replies)?)?;
    let log = decode::<LogBody>(LOG_READ, next(LOG_READ, &mut replies)?)?;

    assert_eq!(
        declined,
        ActReply::FireRefused {
            reason: ShotRefusalNet::MagazineEmpty,
        },
        "a dry gun never reaches the sim, so the reply names the empty magazine — an accepted \
         window would claim the shot was taken: {:?}",
        log.entries,
    );
    let deeds = log.deeds_from(token_of(shot.shooter));
    assert!(
        !deeds.iter().any(|kind| SHOT_DEEDS.contains(kind)),
        "a shot the game turned down is never declared, so the log holds none of it: {deeds:?}",
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
