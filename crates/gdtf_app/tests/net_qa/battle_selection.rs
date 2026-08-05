use bevy::app::App;
use gdtf_app::qa_wire::{
    cell::CellLevelNet,
    misc::ModeKindNet,
    roster::{FactionNet, GangerCardNet},
    token::GangerToken,
};
use gdtf_battle_input::InspectTarget;
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{command::RunOptions, message::QaResponse};
use serde::Deserialize;

use super::{
    battle_reads::an_unreachable_cell,
    battle_setup::battle_with_a_selected_fire_mode,
    command_exchange::{
        BATTLE_ROSTER, BATTLE_SELECTION, BATTLE_TURN, assert_refused_off_the_battle_screen,
        exchange, exchange_all, exchange_expected, ran_body, run,
    },
    socket_support::{TestError, TestResult, battle_app_listening, game_app_listening},
};

#[derive(Debug, Deserialize)]
struct SelectionBody {
    shooter:   Option<GangerToken>,
    fire_mode: Option<ModeKindNet>,
    hovered:   Option<CellLevelNet>,
    pinned:    Option<CellLevelNet>,
}

#[derive(Debug, Deserialize)]
struct RosterBody {
    gangers: Vec<GangerCardNet>,
}

#[derive(Debug, Deserialize)]
struct TurnBody {
    player: Option<FactionNet>,
}

/// A live battle with the inspect target pinned to a cell nothing stands on.
fn pinned_app() -> Result<(App, NetQaPort), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let mut target = InspectTarget::default();
    target.set_pinned(an_unreachable_cell().to_sim());
    app.world_mut().insert_resource(target);
    Ok((app, port))
}

fn decode<T: serde::de::DeserializeOwned>(
    name: &'static str,
    reply: Option<QaResponse>,
) -> Result<T, TestError> {
    let Some(reply) = reply else {
        return Err(format!("`{name}` produced no reply").into());
    };
    let body = ran_body(name, reply)?;
    ron::de::from_str::<T>(&body)
        .map_err(|fault| format!("`{name}`'s body must decode: {fault} — {body}").into())
}

#[test]
fn the_selection_names_a_ganger_the_roster_lists_for_the_player() -> TestResult {
    let replies = exchange_all(
        battle_app_listening,
        vec![
            run(BATTLE_SELECTION, "()", RunOptions::default()),
            run(BATTLE_ROSTER, "()", RunOptions::default()),
            run(BATTLE_TURN, "()", RunOptions::default()),
        ],
    )?;
    let mut replies = replies.into_iter();
    let selection = decode::<SelectionBody>(BATTLE_SELECTION, replies.next())?;
    let roster = decode::<RosterBody>(BATTLE_ROSTER, replies.next())?;
    let turn = decode::<TurnBody>(BATTLE_TURN, replies.next())?;

    let Some(shooter) = selection.shooter else {
        unreachable!("a running battle auto-selects a player ganger: {selection:?}");
    };
    let Some(player) = turn.player else {
        unreachable!("a running battle names the player's gang: {turn:?}");
    };
    let Some(card) = roster.gangers.iter().find(|card| card.token == shooter) else {
        unreachable!("the selected shooter is on the roster: {selection:?} in {roster:?}");
    };
    assert_eq!(
        card.faction, player,
        "the game only ever selects a ganger the player commands: {card:?}",
    );
    assert!(
        selection.fire_mode.is_some(),
        "a selected shooter always carries a fire mode: {selection:?}",
    );
    Ok(())
}

#[test]
fn the_selection_names_the_fire_mode_the_shooter_is_actually_on() -> TestResult {
    let (replies, mode) = exchange_expected(battle_with_a_selected_fire_mode, |_mode| {
        vec![run(BATTLE_SELECTION, "()", RunOptions::default())]
    })?;
    let selection = decode::<SelectionBody>(BATTLE_SELECTION, replies.into_iter().next())?;

    assert_eq!(
        selection.fire_mode,
        Some(mode.kind),
        "the reply reads the live fire mode: this battle was switched off the default \
         before the command ran: {selection:?}",
    );
    Ok(())
}

#[test]
fn the_selection_reports_the_pin_the_inspect_target_holds() -> TestResult {
    let selection = decode::<SelectionBody>(
        BATTLE_SELECTION,
        Some(exchange(
            pinned_app,
            run(BATTLE_SELECTION, "()", RunOptions::default()),
        )?),
    )?;

    assert_eq!(
        selection.pinned,
        Some(an_unreachable_cell()),
        "the reply must carry the live inspect target, not a constant: this battle had its \
         pin set before the command ran: {selection:?}",
    );
    assert_eq!(
        selection.hovered, None,
        "pinning does not invent a hover cell: {selection:?}",
    );
    Ok(())
}

#[test]
fn the_selection_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, BATTLE_SELECTION, "()")?;
    Ok(())
}
