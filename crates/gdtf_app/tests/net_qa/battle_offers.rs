use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::{
    offer::{ContextualActNet, ContextualOfferNet, OfferTargetNet},
    token::GangerToken,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::ganger::Position;
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{command::RunOptions, message::QaResponse};
use serde::Deserialize;

use super::{
    battle_reads::{a_player_ganger, an_enemy_ganger, beside},
    command_exchange::{
        BATTLE_OFFERS, assert_refused_off_the_battle_screen, exchange_expected, ran_body, run,
    },
    socket_support::{TestError, TestResult, battle_app_listening, game_app_listening},
};

#[derive(Debug, Deserialize)]
struct OffersBody {
    offers: Vec<ContextualOfferNet>,
}

/// A live battle with a living enemy standing next to the selected shooter.
fn enemy_adjacent_app() -> Result<(App, NetQaPort, Entity), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let Some((shooter, at)) = a_player_ganger(&app) else {
        return Err("a generated battle must field at least one player ganger".into());
    };
    let Some(enemy) = an_enemy_ganger(&app) else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    app.world_mut()
        .insert_resource(SelectedShooter::new(shooter));
    let next_to = beside(at);
    let Ok(mut row) = app.world_mut().get_entity_mut(enemy) else {
        return Err("the enemy the world just answered with must still exist".into());
    };
    row.insert(Position::new(next_to));
    Ok((app, port, enemy))
}

/// Ask a battle with an adjacent enemy what it offers, and say which enemy that was.
fn offers_beside_an_enemy() -> Result<(OffersBody, Entity), TestError> {
    let (replies, enemy) = exchange_expected(enemy_adjacent_app, |_enemy| {
        vec![run(BATTLE_OFFERS, "()", RunOptions::default())]
    })?;
    let Some(reply) = replies.into_iter().next() else {
        return Err("battle.offers produced no reply".into());
    };
    Ok((offers_body(reply)?, enemy))
}

fn offers_body(reply: QaResponse) -> Result<OffersBody, TestError> {
    let body = ran_body(BATTLE_OFFERS, reply)?;
    ron::de::from_str::<OffersBody>(&body)
        .map_err(|fault| format!("the offers body must decode: {fault} — {body}").into())
}

#[test]
fn an_adjacent_enemy_puts_shove_on_offer_with_that_enemy_as_its_target() -> TestResult {
    let (offers, enemy) = offers_beside_an_enemy()?;

    let shove = offers
        .offers
        .iter()
        .find(|offer| offer.act == ContextualActNet::Shove)
        .ok_or_else(|| -> TestError {
            format!(
                "the panel offers Shove for a living enemy standing 8-adjacent to the selected \
                 shooter, and this read is the same resource the button reads: {offers:?}"
            )
            .into()
        })?;
    assert_eq!(
        shove.target,
        OfferTargetNet::Ganger(GangerToken::new(enemy.to_bits())),
        "the offer carries the enemy the panel picked, minted as its ganger token: {shove:?}",
    );
    Ok(())
}

#[test]
fn offers_come_back_sorted_and_named_from_the_act_families() -> TestResult {
    let (offers, _enemy) = offers_beside_an_enemy()?;

    let acts: Vec<ContextualActNet> = offers.offers.iter().map(|offer| offer.act).collect();
    assert!(
        !acts.is_empty(),
        "an enemy stands beside the selected shooter, so the panel is offering something: \
         {offers:?}",
    );
    assert!(
        acts.is_sorted(),
        "the offers list is sorted before it goes out: {acts:?}",
    );
    let mut unique = acts.clone();
    unique.dedup();
    assert_eq!(
        unique.len(),
        acts.len(),
        "each act family offers at most one button: {acts:?}",
    );
    Ok(())
}

#[test]
fn offers_refuse_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, BATTLE_OFFERS, "()")?;
    Ok(())
}
