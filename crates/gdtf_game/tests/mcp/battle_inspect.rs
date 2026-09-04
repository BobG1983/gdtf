use cobalt_mcp_protocol::{command::RunOptions, message::QaResponse};
use gdtf_game::qa_wire::{
    cell::CellLevelNet,
    inspect::{InspectShownNet, TerrainKindNet},
    roster::GangerCardNet,
    token::GangerToken,
};
use serde::Deserialize;

use super::{
    battle_reads::{a_player_ganger, an_unreachable_cell, cell_argument},
    battle_setup::{
        a_lit_cover_cell, battle_reporting_an_enemy, manned_emplacement_on_authored_terrain,
        shown_fog_lights,
    },
    command_exchange::{
        BATTLE_INSPECT, BATTLE_ROSTER, assert_refused_off_the_battle_screen, exchange_expected,
        exchange_inspecting, exchange_planned, ran_body, run,
    },
    socket_support::{TestError, TestResult, battle_app_listening, game_app_listening},
};

#[derive(Debug, Deserialize)]
struct InspectBody {
    at:    CellLevelNet,
    shown: InspectShownNet,
}

#[derive(Debug, Deserialize)]
struct RosterBody {
    gangers: Vec<GangerCardNet>,
}

fn inspect_body(reply: Option<QaResponse>) -> Result<InspectBody, TestError> {
    let Some(reply) = reply else {
        return Err("battle.inspect produced no reply".into());
    };
    let body = ran_body(BATTLE_INSPECT, reply)?;
    ron::de::from_str::<InspectBody>(&body)
        .map_err(|fault| format!("the inspect body must decode: {fault} — {body}").into())
}

fn roster_body(reply: Option<QaResponse>) -> Result<RosterBody, TestError> {
    let Some(reply) = reply else {
        return Err("battle.roster produced no reply".into());
    };
    let body = ran_body(BATTLE_ROSTER, reply)?;
    ron::de::from_str::<RosterBody>(&body)
        .map_err(|fault| format!("the roster body must decode: {fault} — {body}").into())
}

#[test]
fn inspecting_a_player_ganger_cell_answers_with_that_ganger_card() -> TestResult {
    let replies = exchange_planned(battle_app_listening, |app| {
        let argument =
            a_player_ganger(app).map_or_else(|| "()".to_owned(), |(_, at)| cell_argument(at));
        vec![run(BATTLE_INSPECT, &argument, RunOptions::default())]
    })?;
    let inspect = inspect_body(replies.into_iter().next())?;

    let Some(card) = inspect.shown.ganger else {
        unreachable!(
            "the panel shows a card for a squad-visible ganger, and own-squad members are \
             always squad-visible: {inspect:?}"
        );
    };
    let token: GangerToken = card.token;
    assert!(
        *token > 0,
        "the card carries the ganger's own token: {card:?}",
    );
    Ok(())
}

#[test]
fn inspecting_a_wall_cell_answers_with_the_cover_block_the_panel_draws() -> TestResult {
    let replies = exchange_planned(battle_app_listening, |app| {
        let argument = a_lit_cover_cell(app).map_or_else(|| "()".to_owned(), cell_argument);
        vec![run(BATTLE_INSPECT, &argument, RunOptions::default())]
    })?;
    let inspect = inspect_body(replies.into_iter().next())?;

    let Some(block) = inspect.shown.terrain.as_ref().and_then(|half| half.cover) else {
        unreachable!(
            "the panel draws a cover block for destructible or solid terrain the squad can \
             see, and a generated map always holds some: {inspect:?}"
        );
    };
    assert!(
        *block.hp_max > 0,
        "the block carries the terrain's full-health value: {block:?}",
    );
    Ok(())
}

#[test]
fn inspecting_a_manned_emplacement_answers_with_the_ganger_and_the_terrain_together() -> TestResult
{
    let (app, replies, seat) =
        exchange_inspecting(manned_emplacement_on_authored_terrain, |seat| {
            vec![run(
                BATTLE_INSPECT,
                &cell_argument(seat.at),
                RunOptions::default(),
            )]
        })?;
    assert!(
        *shown_fog_lights(&app, seat.at.to_sim()),
        "the screen must light the seat the shooter is riding, or the terrain half is \
         gated off for a reason this case is not about: {seat:?}",
    );
    let inspect = inspect_body(replies.into_iter().next())?;

    let Some(card) = inspect.shown.ganger.as_ref() else {
        unreachable!("the seat holds the shooter, so the reply carries its card: {inspect:?}");
    };
    assert_eq!(
        card.token,
        GangerToken::new(seat.shooter.to_bits()),
        "the card is the mounted shooter's own: {inspect:?}",
    );
    let Some(terrain) = inspect.shown.terrain.as_ref() else {
        unreachable!(
            "one cell reports everything on it, so the seat answers with its terrain half \
             alongside the card: {inspect:?}"
        );
    };
    assert_eq!(
        terrain.kind,
        TerrainKindNet::Emplacement,
        "the terrain half names the emplacement the shooter is riding: {inspect:?}",
    );
    assert!(
        terrain.state.is_some() && terrain.weapon.is_some(),
        "the terrain half names the seat's state and the weapon it mounts: {inspect:?}",
    );
    Ok(())
}

#[test]
fn the_inspect_read_and_the_roster_agree_about_an_enemy() -> TestResult {
    let (replies, enemy) = exchange_expected(battle_reporting_an_enemy, |enemy| {
        vec![
            run(
                BATTLE_INSPECT,
                &cell_argument(enemy.at),
                RunOptions::default(),
            ),
            run(BATTLE_ROSTER, "()", RunOptions::default()),
        ]
    })?;
    let mut replies = replies.into_iter();
    let inspect = inspect_body(replies.next())?;
    let roster = roster_body(replies.next())?;

    let token = GangerToken::new(enemy.entity.to_bits());
    match roster.gangers.iter().find(|card| card.token == token) {
        Some(card) => assert_eq!(
            inspect.shown.ganger.as_ref(),
            Some(card),
            "the roster lists this enemy, so the panel draws it card for card: {enemy:?} \
             showed as {inspect:?}",
        ),
        None => assert_eq!(
            inspect.shown.ganger, None,
            "the roster leaves this enemy out, so the fog hides it and the panel draws no card \
             for its cell: {enemy:?} showed as {inspect:?}",
        ),
    }
    Ok(())
}

#[test]
fn inspecting_a_cell_off_the_map_shows_nothing() -> TestResult {
    let replies = exchange_planned(battle_app_listening, |_app| {
        vec![run(
            BATTLE_INSPECT,
            &cell_argument(an_unreachable_cell()),
            RunOptions::default(),
        )]
    })?;
    let inspect = inspect_body(replies.into_iter().next())?;

    assert_eq!(
        (
            inspect.shown.ganger.as_ref(),
            inspect.shown.terrain.as_ref()
        ),
        (None, None),
        "no ganger and no terrain stands out there, so the panel would be hidden: {inspect:?}",
    );
    assert_eq!(
        inspect.at,
        an_unreachable_cell(),
        "the reply echoes the cell it was asked about: {inspect:?}",
    );
    Ok(())
}

#[test]
fn inspect_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(
        game_app_listening,
        BATTLE_INSPECT,
        &cell_argument(an_unreachable_cell()),
    )?;
    Ok(())
}
