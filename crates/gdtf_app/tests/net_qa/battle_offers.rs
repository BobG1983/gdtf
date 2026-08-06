use bevy::ecs::entity::Entity;
use gdtf_app::qa_wire::{
    offer::{ContextualActNet, ContextualOfferNet, OfferTargetNet},
    token::GangerToken,
};
use gdtf_qa_protocol::command::RunOptions;
use serde::Deserialize;

use super::{
    act_support::{caught_up, decode, next, selected},
    battle_reads::{ganger_argument, token_of},
    battle_setup::battle_with_an_enemy_beside_an_idle_ganger,
    command_exchange::{
        ACT_SELECT, BATTLE_OFFERS, WAIT, assert_refused_off_the_battle_screen, exchange_expected,
        run,
    },
    socket_support::{TestError, TestResult, game_app_listening},
};

#[derive(Debug, Deserialize)]
struct OffersBody {
    offers: Vec<ContextualOfferNet>,
}

/// Read the panel either side of an `act.select`, and say which enemy the shooter was placed by.
fn offers_beside_an_enemy() -> Result<(OffersBody, Entity), TestError> {
    let (replies, adjacent) =
        exchange_expected(battle_with_an_enemy_beside_an_idle_ganger, |adjacent| {
            vec![
                caught_up(),
                run(BATTLE_OFFERS, "()", RunOptions::default()),
                run(
                    ACT_SELECT,
                    &ganger_argument(adjacent.shooter),
                    RunOptions::default(),
                ),
                run(BATTLE_OFFERS, "()", RunOptions::default()),
            ]
        })?;
    let mut replies = replies.into_iter();
    let _caught = next(WAIT, &mut replies)?;
    let before = decode::<OffersBody>(BATTLE_OFFERS, next(BATTLE_OFFERS, &mut replies)?)?;
    let shooter = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let after = decode::<OffersBody>(BATTLE_OFFERS, next(BATTLE_OFFERS, &mut replies)?)?;

    let against_the_enemy = OfferTargetNet::Ganger(GangerToken::new(adjacent.enemy.to_bits()));
    assert!(
        !before
            .offers
            .iter()
            .any(|offer| offer.target == against_the_enemy),
        "the enemy stands beside a ganger the game picked for itself, so nothing may offer \
         against it until act.select names the shooter it stands next to, or this case would \
         read the same with no selection command at all: {before:?}",
    );
    assert_eq!(
        shooter,
        Some(token_of(adjacent.shooter)),
        "act.select answers with the ganger it selected: {shooter:?}",
    );
    Ok((after, adjacent.enemy))
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
fn the_panel_offers_something_once_act_select_names_the_shooter() -> TestResult {
    let (offers, _enemy) = offers_beside_an_enemy()?;

    assert!(
        !offers.offers.is_empty(),
        "an enemy stands beside the selected shooter, so the panel is offering something: \
         {offers:?}",
    );
    Ok(())
}

#[test]
fn offers_refuse_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, BATTLE_OFFERS, "()")?;
    Ok(())
}
