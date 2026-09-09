//! Clicks over a real socket: one selects, one pins an enemy's cell, two in a row walk a ganger.

use bevy::ecs::entity::Entity;
use cobalt_mcp_protocol::command::RunOptions;
use gdtf_game::qa_wire::{
    act::ActReply,
    cell::CellLevelNet,
    click::{ClickDecisionNet, ClickReply},
    token::GangerToken,
};

use super::{
    act_support::{assert_caught_up, battle_app_reporting, caught_up, decode, next, selected},
    battle_reads::{
        a_player_ganger, an_enemy_ganger_at, an_unreachable_cell, an_unselected_player_ganger,
        cell_argument, cell_of, ganger_argument, one_step_from_without_a_link, token_of,
    },
    command_exchange::{
        ACT_SELECT, BATTLE_SELECTION, INPUT_CLICK_CELL, WAIT, assert_refused_off_the_battle_screen,
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

/// Why the fixture cannot host the walking case.
const NO_STEP: &str =
    "the generated map must offer one clear step off the ganger with no vertical link on it";

/// The ganger a click case picks, and the cell its sprite stands on.
struct ClickTarget {
    token: GangerToken,
    at:    CellLevelNet,
}

/// The ganger the two-click case walks, and the cell both clicks land on.
struct ClickWalk {
    ganger: Entity,
    step:   CellLevelNet,
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
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let clicked = decode::<ClickReply>(INPUT_CLICK_CELL, next(INPUT_CLICK_CELL, &mut replies)?)?;
    let selection =
        decode::<ClickedSelectionBody>(BATTLE_SELECTION, next(BATTLE_SELECTION, &mut replies)?)?;

    assert_eq!(
        selection.shooter,
        Some(target.token),
        "the click ran the game's own left-click decision on that cell, which selects the player \
         ganger standing on it — {clicked:?}",
    );
    assert_eq!(
        clicked,
        ClickReply {
            decision: ClickDecisionNet::Select,
            act:      None,
        },
        "the reply names the decision the game made, and a click that only selects pushes no act \
         to report on",
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
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let clicked = decode::<ClickReply>(INPUT_CLICK_CELL, next(INPUT_CLICK_CELL, &mut replies)?)?;
    let selection =
        decode::<ClickedSelectionBody>(BATTLE_SELECTION, next(BATTLE_SELECTION, &mut replies)?)?;

    assert_eq!(
        selection.pinned,
        Some(at),
        "the same click also runs the pin decision, which pins the cell a ganger the player does \
         not command stands on — that pin is what the inspect panel then reads: {clicked:?}",
    );
    assert_eq!(
        clicked.act.is_some(),
        clicked.decision == ClickDecisionNet::Fire || clicked.decision == ClickDecisionNet::Move,
        "the reply carries an act reply exactly when the decision pushed an act: {clicked:?}",
    );
    Ok(())
}

#[test]
fn a_second_click_on_the_pinned_cell_moves_the_ganger_and_reports_the_move_it_ran() -> TestResult {
    let (replies, walk) = exchange_expected(
        battle_app_reporting(
            |app| {
                let (ganger, at) = a_player_ganger(app)?;
                Some(ClickWalk {
                    ganger,
                    step: one_step_from_without_a_link(app, at)?,
                })
            },
            NO_STEP,
        ),
        |walk| {
            vec![
                caught_up(),
                run(
                    ACT_SELECT,
                    &ganger_argument(walk.ganger),
                    RunOptions::default(),
                ),
                run(
                    INPUT_CLICK_CELL,
                    &cell_argument(walk.step),
                    RunOptions::default(),
                ),
                run(
                    INPUT_CLICK_CELL,
                    &cell_argument(walk.step),
                    RunOptions::default(),
                ),
            ]
        },
    )?;
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let shooter = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let pinned = decode::<ClickReply>(INPUT_CLICK_CELL, next(INPUT_CLICK_CELL, &mut replies)?)?;
    let walked = decode::<ClickReply>(INPUT_CLICK_CELL, next(INPUT_CLICK_CELL, &mut replies)?)?;

    assert_eq!(
        shooter,
        Some(token_of(walk.ganger)),
        "the walk comes from the ganger the token named, so selection must have taken first",
    );
    assert_eq!(
        pinned.decision,
        ClickDecisionNet::SetMoveTarget,
        "the first click on a clear step pins it as the move target rather than walking it: \
         {pinned:?}",
    );
    assert_eq!(
        pinned.act, None,
        "pinning a target pushes no act, so there is nothing for the reply to report on: \
         {pinned:?}",
    );
    assert_eq!(
        walked.decision,
        ClickDecisionNet::Move,
        "the second click on the same cell confirms the pinned move: {walked:?}",
    );
    assert!(
        matches!(walked.act, Some(ActReply::Accepted { .. })),
        "a click the game decided as Move pushes the move act, and the reply carries what that \
         act answered rather than leaving the caller to read the log: {walked:?}",
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
