//! The visible, roster and inspect reads answer with the picture on screen.

use cobalt_mcp_protocol::{
    command::RunOptions,
    message::{QaRequest, QaResponse},
};
use gdtf_game::qa_wire::{
    inspect::InspectShownNet, roster::GangerCardNet, token::GangerToken, visible::VisibleGangerNet,
};
use serde::Deserialize;

use crate::{
    battle_reads::cell_argument,
    battle_setup::{
        Standing, battle_with_a_frozen_fog, battle_with_a_ganger_the_screen_has_not_moved,
        battle_with_an_occupant_the_screen_has_not_seen,
    },
    command_exchange::{
        BATTLE_INSPECT, BATTLE_ROSTER, BATTLE_VISIBLE, exchange_expected, ran_body, run,
    },
    socket_support::{TestError, TestResult},
};

#[derive(Debug, Deserialize)]
struct VisibleBody {
    enemies: Vec<VisibleGangerNet>,
}

#[derive(Debug, Deserialize)]
struct RosterBody {
    gangers: Vec<GangerCardNet>,
}

#[derive(Debug, Deserialize)]
struct InspectBody {
    shown: InspectShownNet,
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

fn both_reads() -> Vec<QaRequest> {
    vec![
        run(BATTLE_VISIBLE, "()", RunOptions::default()),
        run(BATTLE_ROSTER, "()", RunOptions::default()),
    ]
}

fn split(replies: Vec<QaResponse>) -> Result<(VisibleBody, RosterBody), TestError> {
    let mut replies = replies.into_iter();
    let visible = decode::<VisibleBody>(BATTLE_VISIBLE, replies.next())?;
    let roster = decode::<RosterBody>(BATTLE_ROSTER, replies.next())?;
    Ok((visible, roster))
}

#[test]
fn an_enemy_is_read_where_the_screen_draws_it_not_where_the_sim_moved_it() -> TestResult {
    let (replies, enemy) = exchange_expected(
        || battle_with_a_ganger_the_screen_has_not_moved(Standing::Lit),
        |_enemy| both_reads(),
    )?;
    let (visible, roster) = split(replies)?;
    let token = GangerToken::new(enemy.entity.to_bits());

    let Some(entry) = visible.enemies.iter().find(|entry| entry.token == token) else {
        unreachable!(
            "the sprite still stands in the lit area, so the read lists it: {enemy:?} missing \
             from {visible:?}"
        );
    };
    assert_eq!(
        entry.at, enemy.drawn,
        "the entry names the cell the sprite is drawn on, not the one the sim moved it to: \
         {entry:?} against {enemy:?}",
    );
    assert_ne!(
        enemy.drawn, enemy.live,
        "the fixture must really split the two cells, or this case proves nothing",
    );
    let Some(card) = roster.gangers.iter().find(|card| card.token == token) else {
        unreachable!(
            "the roster judges from the same drawn cell, so it lists what the screen shows too: \
             {enemy:?} — {roster:?}"
        );
    };
    assert_eq!(
        card.at, enemy.drawn,
        "the card places the ganger on the cell its sprite is drawn on, the same cell the lit \
         area named: {card:?} against {enemy:?}",
    );
    Ok(())
}

#[test]
fn an_enemy_the_screen_still_hides_stays_out_of_both_reads() -> TestResult {
    let (replies, enemy) = exchange_expected(
        || battle_with_a_ganger_the_screen_has_not_moved(Standing::Hidden),
        |_enemy| both_reads(),
    )?;
    let (visible, roster) = split(replies)?;
    let token = GangerToken::new(enemy.entity.to_bits());

    assert!(
        !visible.enemies.iter().any(|entry| entry.token == token),
        "the sim has walked it into the light but the sprite is still in the dark, so the lit \
         area leaves it out: {enemy:?} appears in {visible:?}",
    );
    assert!(
        !roster.gangers.iter().any(|card| card.token == token),
        "the roster leaves it out for the same reason: {enemy:?} appears in {roster:?}",
    );
    Ok(())
}

#[test]
fn a_cell_the_screens_fog_still_lights_reads_as_lit() -> TestResult {
    let (replies, enemy) = exchange_expected(battle_with_a_frozen_fog, |_enemy| both_reads())?;
    let (visible, roster) = split(replies)?;
    let token = GangerToken::new(enemy.entity.to_bits());

    let Some(entry) = visible.enemies.iter().find(|entry| entry.token == token) else {
        unreachable!(
            "the screen's fog still lights that cell, so the enemy standing on it is in the \
             lit area: {enemy:?} missing from {visible:?}"
        );
    };
    assert_eq!(
        entry.at, enemy.at,
        "the entry names the cell it stands on: {entry:?}",
    );
    assert!(
        roster.gangers.iter().any(|card| card.token == token),
        "the roster reads the same frozen fog, so the enemy still has a card: {enemy:?} — \
         {roster:?}",
    );
    Ok(())
}

#[test]
fn the_inspect_read_ignores_an_occupant_the_screen_has_not_shown_arriving() -> TestResult {
    let (replies, occupant) = exchange_expected(
        battle_with_an_occupant_the_screen_has_not_seen,
        |occupant| {
            vec![run(
                BATTLE_INSPECT,
                &cell_argument(occupant.at),
                RunOptions::default(),
            )]
        },
    )?;
    let inspect = decode::<InspectBody>(BATTLE_INSPECT, replies.into_iter().next())?;

    assert_eq!(
        inspect.shown.ganger, None,
        "the panel draws no card on a cell whose occupant the screen has not shown arriving: \
         {occupant:?} showed as {inspect:?}",
    );
    Ok(())
}
