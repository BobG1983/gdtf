use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_protocol::{command::RunOptions, ports::McpPort};
use gdtf_battle_sim::{acts::shove_tu_cost, ganger::Tu, tuning::CombatTuning};
use gdtf_game::qa_wire::{
    offer::{ContextualActNet, ContextualOfferNet, OfferPressableNet, OfferTargetNet},
    token::GangerToken,
};
use serde::Deserialize;

use super::{
    act_support::{assert_caught_up, caught_up, decode, next, selected},
    battle_reads::{ganger_argument, token_of},
    battle_setup::{
        IdlePair, battle_with_an_enemy_beside_an_idle_ganger, let_the_screen_catch_up,
        the_screen_has_caught_up,
    },
    command_exchange::{
        ACT_SELECT, ACT_SHOVE, BATTLE_OFFERS, WAIT, assert_refused_off_the_battle_screen,
        exchange_expected, exchange_inspecting, run,
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
    assert_caught_up(next(WAIT, &mut replies)?)?;
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

/// What one shove charges, from the sim's own cost helper against the live tuning.
fn shove_cost(app: &App) -> Result<Tu, TestError> {
    app.world()
        .get_resource::<CombatTuning>()
        .map(shove_tu_cost)
        .ok_or_else(|| "a running battle must hold the combat tuning a shove is priced from".into())
}

/// What the wire said about the shove offer, and what the shover's pool did around the call.
struct ShoveRun {
    pressable: OfferPressableNet,
    cost:      Tu,
    before:    Tu,
    after:     Tu,
}

/// The battle a shove case runs in: the shover's pool written, and the screen level with it.
fn battle_with_a_pool_for_one_shove(
    short_by_one: bool,
) -> Result<(App, McpPort, IdlePair), TestError> {
    let (mut app, port, adjacent) = battle_with_an_enemy_beside_an_idle_ganger()?;
    let cost = shove_cost(&app)?;
    let pool = if short_by_one {
        Tu::new(cost.saturating_sub(1))
    } else {
        cost
    };
    let Ok(mut row) = app.world_mut().get_entity_mut(adjacent.shooter) else {
        return Err("the idle ganger the fixture reported must still exist".into());
    };
    row.insert(pool);
    let_the_screen_catch_up(&mut app);
    Ok((app, port, adjacent))
}

/// Select the idle ganger with `short_by_one` deciding its pool, read the offer, then shove.
fn a_shove_at_a_pool(short_by_one: bool) -> Result<ShoveRun, TestError> {
    let (mut app, replies, adjacent) = exchange_inspecting(
        move || battle_with_a_pool_for_one_shove(short_by_one),
        |adjacent| {
            vec![
                caught_up(),
                run(
                    ACT_SELECT,
                    &ganger_argument(adjacent.shooter),
                    RunOptions::default(),
                ),
                run(BATTLE_OFFERS, "()", RunOptions::default()),
                run(ACT_SHOVE, "()", RunOptions::default()),
            ]
        },
    )?;
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let _shooter = selected(ACT_SELECT, next(ACT_SELECT, &mut replies)?)?;
    let offers = decode::<OffersBody>(BATTLE_OFFERS, next(BATTLE_OFFERS, &mut replies)?)?;
    let _shoved = next(ACT_SHOVE, &mut replies)?;
    let_the_screen_catch_up(&mut app);

    let cost = shove_cost(&app)?;
    let Some(shove) = offers
        .offers
        .iter()
        .find(|offer| offer.act == ContextualActNet::Shove)
    else {
        return Err(format!(
            "the panel offers Shove whether or not the shover can pay for it, so the offer must \
             be on the list either way: {offers:?}"
        )
        .into());
    };
    assert_eq!(
        shove.target,
        OfferTargetNet::Ganger(GangerToken::new(adjacent.enemy.to_bits())),
        "the offer names the enemy the fixture stood beside the shooter",
    );
    let Some(after) = app.world().get::<Tu>(adjacent.shooter).copied() else {
        return Err("the shover must still carry a TU pool after the call".into());
    };
    let before = if short_by_one {
        Tu::new(cost.saturating_sub(1))
    } else {
        cost
    };
    Ok(ShoveRun {
        pressable: shove.pressable,
        cost,
        before,
        after,
    })
}

#[test]
fn a_pool_below_the_cost_is_offered_as_not_pressable_and_the_dispatch_does_nothing() -> TestResult {
    let probe = a_shove_at_a_pool(true)?;
    assert!(
        *probe.cost > 0,
        "a shove must cost something or a pool below it cannot exist",
    );
    assert_eq!(
        probe.pressable,
        OfferPressableNet::new(false),
        "the wire reports the greyed button the panel is showing, not a second affordability call",
    );
    assert_eq!(
        probe.after, probe.before,
        "act.shove on an offer the wire called not pressable spends nothing, so the two agree",
    );
    Ok(())
}

#[test]
fn a_pool_that_covers_the_cost_is_offered_as_pressable_and_the_dispatch_charges_it() -> TestResult {
    let probe = a_shove_at_a_pool(false)?;
    assert!(
        *probe.cost > 0,
        "a shove must cost something or this case cannot tell a charge from a refusal",
    );
    assert_eq!(
        probe.pressable,
        OfferPressableNet::new(true),
        "a pool that covers the cost leaves the button pressable on the wire",
    );
    assert_eq!(
        (*probe.before).checked_sub(*probe.after),
        Some(*probe.cost),
        "act.shove on an offer the wire called pressable charges exactly the sim's own quote",
    );
    Ok(())
}

#[test]
fn the_shove_fixture_hands_over_a_screen_the_act_log_cannot_get_ahead_of() -> TestResult {
    let (mut app, _port, _adjacent) = battle_with_a_pool_for_one_shove(false)?;

    assert!(
        the_screen_has_caught_up(&mut app),
        "the client's first act is admitted against the screen, so the fixture may not hand \
         over one that is still playing the log back",
    );
    app.update();
    assert!(
        the_screen_has_caught_up(&mut app),
        "the pool the fixture wrote is already logged and played, so the next frame has \
         nothing left to put the screen behind with — the frame a command arrives on decides \
         whether it is refused",
    );
    Ok(())
}

#[test]
fn offers_refuse_off_the_battle_screen() -> TestResult {
    assert_refused_off_the_battle_screen(game_app_listening, BATTLE_OFFERS, "()")?;
    Ok(())
}
