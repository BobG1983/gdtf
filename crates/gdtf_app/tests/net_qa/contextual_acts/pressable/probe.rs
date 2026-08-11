//! Reading one act's offer off a live battle whose shooter holds a chosen TU pool.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::offer::{ContextualActNet, ContextualOfferNet};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{ganger::Tu, tuning::CombatTuning};
use gdtf_qa_protocol::{command::RunOptions, ports::NetQaPort};
use serde::Deserialize;

use crate::{
    act_support::{assert_caught_up, caught_up, decode, next},
    command_exchange::{BATTLE_OFFERS, WAIT, exchange_expected, run},
    contextual_acts::scene::settle,
    socket_support::TestError,
};

/// The body `battle.offers` answers with, as far as these cases read it.
#[derive(Debug, Deserialize)]
struct OffersBody {
    offers: Vec<ContextualOfferNet>,
}

/// The pool a case writes onto the shooter, priced from the act's own sim helper.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Pool {
    /// One TU short of the quote.
    ShortOfTheQuote,
    /// Exactly the quote.
    TheQuote,
}

/// One act's offer, and the quote the shooter's pool was written against.
#[derive(Debug)]
pub(crate) struct OfferAtPool {
    /// What the panel was offering for that act.
    pub(crate) offer: ContextualOfferNet,
    /// What the sim's own helper charges for it.
    pub(crate) cost:  Tu,
}

/// A quote the combat tuning alone prices, read off the live world.
pub(crate) fn from_tuning(price: fn(&CombatTuning) -> Tu) -> impl Fn(&App) -> Option<Tu> {
    move |app| app.world().get_resource::<CombatTuning>().map(price)
}

/// Who the fixture selected before it handed the battle over.
pub(crate) fn shooter(app: &App) -> Option<Entity> {
    app.world()
        .get_resource::<SelectedShooter>()
        .and_then(|selected| **selected)
}

/// Read one act's offer off a battle whose shooter holds `pool`, priced by `quote`.
pub(crate) fn offer_at_pool<T>(
    act: ContextualActNet,
    scene: impl FnOnce() -> Result<(App, NetQaPort, T), TestError>,
    quote: impl Fn(&App) -> Option<Tu>,
    pool: Pool,
) -> Result<OfferAtPool, TestError> {
    let (offer, cost) = read_offer(act, || {
        let (mut app, port, _carried) = scene()?;
        let cost = write_pool(&mut app, &quote, pool)?;
        Ok((app, port, cost))
    })?;
    Ok(OfferAtPool { offer, cost })
}

/// Read one act's offer off a battle the case shaped for itself.
pub(crate) fn offer_of<T>(
    act: ContextualActNet,
    scene: impl FnOnce() -> Result<(App, NetQaPort, T), TestError>,
) -> Result<ContextualOfferNet, TestError> {
    let (offer, _carried) = read_offer(act, scene)?;
    Ok(offer)
}

/// Ask the running battle what the panel is offering, and pick `act` out of the answer.
fn read_offer<T>(
    act: ContextualActNet,
    scene: impl FnOnce() -> Result<(App, NetQaPort, T), TestError>,
) -> Result<(ContextualOfferNet, T), TestError> {
    let (replies, carried) = exchange_expected(scene, |_carried| {
        vec![caught_up(), run(BATTLE_OFFERS, "()", RunOptions::default())]
    })?;
    let mut replies = replies.into_iter();
    assert_caught_up(next(WAIT, &mut replies)?)?;
    let body = decode::<OffersBody>(BATTLE_OFFERS, next(BATTLE_OFFERS, &mut replies)?)?;
    let offer = body
        .offers
        .iter()
        .find(|offer| offer.act == act)
        .copied()
        .ok_or_else(|| -> TestError {
            format!(
                "the panel offers {act:?} whether or not the actor can pay for it, so the offer \
                 must be on the list either way: {body:?}"
            )
            .into()
        })?;
    Ok((offer, carried))
}

/// Write the pool `pool` names onto the shooter, and report the quote it was priced against.
fn write_pool(
    app: &mut App,
    quote: &impl Fn(&App) -> Option<Tu>,
    pool: Pool,
) -> Result<Tu, TestError> {
    let Some(cost) = quote(app) else {
        return Err("a running battle must price this act with the sim's own cost helper".into());
    };
    let Some(shooter) = shooter(app) else {
        return Err("the fixture must hand over a battle with a shooter selected".into());
    };
    let written = match pool {
        Pool::ShortOfTheQuote => Tu::new(cost.saturating_sub(1)),
        Pool::TheQuote => cost,
    };
    let Ok(mut row) = app.world_mut().get_entity_mut(shooter) else {
        return Err("the selected shooter must still exist to be given a pool".into());
    };
    row.insert(written);
    settle(app);
    Ok(cost)
}
