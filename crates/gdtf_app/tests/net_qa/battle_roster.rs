use gdtf_app::qa_wire::{
    roster::{FactionNet, GangerCardNet},
    token::GangerToken,
    visible::VisibleGangerNet,
};
use gdtf_qa_protocol::{command::RunOptions, message::QaResponse};
use serde::Deserialize;

use super::{
    battle_setup::{ExpectedEnemy, Standing, battle_reporting_a_player_card, battle_with_an_enemy},
    command_exchange::{
        BATTLE_ROSTER, BATTLE_TURN, BATTLE_VISIBLE, assert_refused_off_the_battle_screen,
        exchange_all, exchange_expected, ran_body, run,
    },
    socket_support::{TestError, TestResult, battle_app_listening, game_app_listening},
};

#[derive(Debug, Deserialize)]
struct RosterBody {
    gangers: Vec<GangerCardNet>,
}

#[derive(Debug, Deserialize)]
struct TurnBody {
    player: Option<FactionNet>,
}

#[derive(Debug, Deserialize)]
struct VisibleBody {
    enemies: Vec<VisibleGangerNet>,
}

fn roster_turn_and_lit_area() -> Result<(RosterBody, TurnBody, VisibleBody), TestError> {
    let replies = exchange_all(
        battle_app_listening,
        vec![
            run(BATTLE_ROSTER, "()", RunOptions::default()),
            run(BATTLE_TURN, "()", RunOptions::default()),
            run(BATTLE_VISIBLE, "()", RunOptions::default()),
        ],
    )?;
    let mut replies = replies.into_iter();
    let roster = decode_roster(replies.next())?;
    let turn = decode_turn(replies.next())?;
    let visible = decode_visible(replies.next())?;
    Ok((roster, turn, visible))
}

fn decode_visible(reply: Option<QaResponse>) -> Result<VisibleBody, TestError> {
    let Some(reply) = reply else {
        return Err("battle.visible produced no reply".into());
    };
    let body = ran_body(BATTLE_VISIBLE, reply)?;
    ron::de::from_str::<VisibleBody>(&body).map_err(|fault| {
        format!("the lit-area body must decode into its published shape: {fault} — {body}").into()
    })
}

fn decode_roster(reply: Option<QaResponse>) -> Result<RosterBody, TestError> {
    let Some(reply) = reply else {
        return Err("battle.roster produced no reply".into());
    };
    let body = ran_body(BATTLE_ROSTER, reply)?;
    ron::de::from_str::<RosterBody>(&body).map_err(|fault| {
        format!("the roster body must decode into its published shape: {fault} — {body}").into()
    })
}

fn decode_turn(reply: Option<QaResponse>) -> Result<TurnBody, TestError> {
    let Some(reply) = reply else {
        return Err("battle.turn produced no reply".into());
    };
    let body = ran_body(BATTLE_TURN, reply)?;
    ron::de::from_str::<TurnBody>(&body).map_err(|fault| {
        format!("the turn body must decode into its published shape: {fault} — {body}").into()
    })
}

#[test]
fn the_roster_carries_the_player_squad_and_only_visible_enemies() -> TestResult {
    let (roster, turn, visible) = roster_turn_and_lit_area()?;

    assert!(
        !roster.gangers.is_empty(),
        "a generated battle always fields gangers: {roster:?}",
    );
    let Some(player) = turn.player else {
        unreachable!("a running battle names the faction the player commands: {turn:?}");
    };
    assert!(
        roster.gangers.iter().any(|card| card.faction == player),
        "every player ganger is on the roster, so at least one card carries the player \
         faction: {roster:?}",
    );
    for card in &roster.gangers {
        if card.faction != player {
            assert!(
                visible
                    .enemies
                    .iter()
                    .any(|entry| entry.token == card.token),
                "an enemy reaches the roster only while the lit area holds it too, because both \
                 reads judge from the same shown fog: {card:?} against {visible:?}",
            );
        }
    }
    Ok(())
}

/// Ask for the roster in a battle where one enemy has been stood at a known cell.
fn roster_with_an_enemy(standing: Standing) -> Result<(RosterBody, ExpectedEnemy), TestError> {
    let (replies, enemy) = exchange_expected(
        move || battle_with_an_enemy(standing),
        |_enemy| vec![run(BATTLE_ROSTER, "()", RunOptions::default())],
    )?;
    let roster = decode_roster(replies.into_iter().next())?;
    Ok((roster, enemy))
}

#[test]
fn an_enemy_the_squad_can_see_gets_a_card_of_its_own() -> TestResult {
    let (roster, enemy) = roster_with_an_enemy(Standing::Lit)?;
    let token = GangerToken::new(enemy.entity.to_bits());

    let Some(card) = roster.gangers.iter().find(|card| card.token == token) else {
        unreachable!(
            "an enemy standing on a lit cell is on the roster: {enemy:?} missing from {roster:?}"
        );
    };
    assert!(
        *card.hp_max > 0,
        "an enemy card carries the same stat block a player card does — the reduction is which \
         enemies appear, not which fields they carry: {card:?}",
    );
    Ok(())
}

#[test]
fn an_enemy_the_fog_hides_is_left_off_the_roster() -> TestResult {
    let (roster, enemy) = roster_with_an_enemy(Standing::Hidden)?;
    let token = GangerToken::new(enemy.entity.to_bits());

    assert!(
        !roster.gangers.iter().any(|card| card.token == token),
        "an enemy the fog hides is not on the roster at all, so a client cannot read it \
         through this command: {enemy:?} appears in {roster:?}",
    );
    Ok(())
}

#[test]
fn a_card_carries_the_ganger_the_world_holds_field_for_field() -> TestResult {
    let (replies, live) = exchange_expected(battle_reporting_a_player_card, |_live| {
        vec![run(BATTLE_ROSTER, "()", RunOptions::default())]
    })?;
    let roster = decode_roster(replies.into_iter().next())?;

    let Some(card) = roster.gangers.iter().find(|card| card.token == live.token) else {
        unreachable!("every player ganger is on the roster: {live:?} missing from {roster:?}");
    };
    assert_eq!(
        card.name, live.name,
        "the card carries the ganger's own name, read from the live world: {card:?}",
    );
    assert_eq!(
        card.faction, live.faction,
        "the card carries the gang the live world put it in: {card:?}",
    );
    assert_eq!(
        card.tu_max, live.tu_max,
        "the card's tu_max is the ganger's live TuMax, not its current TU: {card:?}",
    );
    if let Some(hp_max) = live.hp_max {
        assert_eq!(
            card.hp_max, hp_max,
            "the card's hp_max is the ganger's live HpMax: {card:?}",
        );
    }
    assert!(
        *card.tu <= *card.tu_max && *card.hp <= *card.hp_max,
        "a card's current values never exceed its maxima: {card:?}",
    );
    Ok(())
}

#[test]
fn the_roster_is_ordered_so_a_polling_client_sees_one_list() -> TestResult {
    let (roster, _turn, _visible) = roster_turn_and_lit_area()?;
    let tokens: Vec<u64> = roster.gangers.iter().map(|card| *card.token).collect();
    assert!(
        tokens.is_sorted(),
        "the roster is sorted before it goes out: {tokens:?}",
    );
    let mut unique = tokens.clone();
    unique.dedup();
    assert_eq!(
        unique.len(),
        tokens.len(),
        "each ganger appears once: {tokens:?}",
    );
    Ok(())
}

#[test]
fn the_roster_refuses_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, BATTLE_ROSTER, "()")?;
    Ok(())
}
