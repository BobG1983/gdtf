//! Clicks over a real socket: one selects a player ganger, one pins the cell an enemy stands on.

use gdtf_app::qa_wire::{cell::CellLevelNet, token::GangerToken};
use gdtf_qa_protocol::command::RunOptions;

use super::{
    act_support::{accepted, battle_app_reporting, caught_up, decode, next},
    battle_reads::{
        an_enemy_ganger_at, an_unreachable_cell, an_unselected_player_ganger, cell_argument,
        cell_of, token_of,
    },
    command_exchange::{
        BATTLE_SELECTION, INPUT_CLICK_CELL, WAIT, assert_refused_off_the_battle_screen,
        exchange_expected, run,
    },
    input_support::ClickedSelectionBody,
    socket_support::{TestResult, game_app_listening},
};

/// Why the fixture cannot host the selecting case.
const NO_UNSELECTED: &str =
    "the fixture must hold a player ganger the game has not already selected";

/// Why the fixture cannot host the pinning case.
const NO_ENEMY: &str = "the fixture must hold a living enemy ganger to click on";

/// The ganger a click case picks, and the cell its sprite stands on.
struct ClickTarget {
    token: GangerToken,
    at:    CellLevelNet,
}

#[test]
fn a_click_on_a_player_ganger_selects_it() -> TestResult {
    let (replies, target) = exchange_expected(
        battle_app_reporting(
            |app| {
                let entity = an_unselected_player_ganger(app)?;
                Some(ClickTarget {
                    token: token_of(entity),
                    at:    cell_of(app, entity)?,
                })
            },
            NO_UNSELECTED,
        ),
        |target| {
            vec![
                caught_up(),
                run(
                    INPUT_CLICK_CELL,
                    &cell_argument(target.at),
                    RunOptions::default(),
                ),
                run(BATTLE_SELECTION, "()", RunOptions::default()),
            ]
        },
    )?;
    let mut replies = replies.into_iter();
    let _waited = next(WAIT, &mut replies)?;
    let clicked = accepted(INPUT_CLICK_CELL, next(INPUT_CLICK_CELL, &mut replies)?)?;
    let selection =
        decode::<ClickedSelectionBody>(BATTLE_SELECTION, next(BATTLE_SELECTION, &mut replies)?)?;

    assert_eq!(
        selection.shooter,
        Some(target.token),
        "the click ran the game's own left-click decision on that cell, which selects the player \
         ganger standing on it — {clicked:?}",
    );
    Ok(())
}

#[test]
fn a_click_on_an_enemy_pins_the_cell_it_stands_on() -> TestResult {
    let (replies, at) = exchange_expected(
        battle_app_reporting(|app| an_enemy_ganger_at(app).map(|(_, at)| at), NO_ENEMY),
        |at| {
            vec![
                caught_up(),
                run(INPUT_CLICK_CELL, &cell_argument(*at), RunOptions::default()),
                run(BATTLE_SELECTION, "()", RunOptions::default()),
            ]
        },
    )?;
    let mut replies = replies.into_iter();
    let _waited = next(WAIT, &mut replies)?;
    let clicked = accepted(INPUT_CLICK_CELL, next(INPUT_CLICK_CELL, &mut replies)?)?;
    let selection =
        decode::<ClickedSelectionBody>(BATTLE_SELECTION, next(BATTLE_SELECTION, &mut replies)?)?;

    assert_eq!(
        selection.pinned,
        Some(at),
        "the same click also runs the pin decision, which pins the cell a ganger the player does \
         not command stands on — that pin is what the inspect panel then reads: {clicked:?}",
    );
    Ok(())
}

#[test]
fn a_click_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(
        game_app_listening,
        INPUT_CLICK_CELL,
        &cell_argument(an_unreachable_cell()),
    )
}
