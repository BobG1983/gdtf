//! Execute and stabilize grey out when the pool is short of what the sim prices them at.

use gdtf_battle_sim::acts::downed::{execute_tu_cost, stabilize_tu_cost};
use gdtf_game::qa_wire::offer::{ContextualActNet, OfferPressableNet};

use super::probe::{OfferAtPool, Pool, from_tuning, offer_at_pool};
use crate::{
    contextual_acts::neighbour::{
        bleeding_mate_beside_the_shooter, downed_enemy_beside_the_shooter,
    },
    socket_support::{TestError, TestResult},
};

/// The execute offer, read with the shooter's pool set from `pool`.
fn execute_offer(pool: Pool) -> Result<OfferAtPool, TestError> {
    offer_at_pool(
        ContextualActNet::Execute,
        downed_enemy_beside_the_shooter,
        from_tuning(execute_tu_cost),
        pool,
    )
}

/// The stabilize offer, read with the shooter's pool set from `pool`.
fn stabilize_offer(pool: Pool) -> Result<OfferAtPool, TestError> {
    offer_at_pool(
        ContextualActNet::Stabilize,
        bleeding_mate_beside_the_shooter,
        from_tuning(stabilize_tu_cost),
        pool,
    )
}

#[test]
fn a_pool_below_the_execute_quote_is_offered_as_not_pressable() -> TestResult {
    let probe = execute_offer(Pool::ShortOfTheQuote)?;

    assert!(
        *probe.cost > 0,
        "an execute must cost something or a pool below it cannot exist",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(false),
        "the wire reports the greyed button the panel is showing for a pool one short of the \
         sim's own execute quote: {probe:?}",
    );
    Ok(())
}

#[test]
fn a_pool_that_covers_the_execute_quote_is_offered_as_pressable() -> TestResult {
    let probe = execute_offer(Pool::TheQuote)?;

    assert!(
        *probe.cost > 0,
        "an execute must cost something or this case cannot tell a greyed button from a live one",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(true),
        "a pool that covers the quote leaves the execute button pressable: {probe:?}",
    );
    Ok(())
}

#[test]
fn a_pool_below_the_stabilize_quote_is_offered_as_not_pressable() -> TestResult {
    let probe = stabilize_offer(Pool::ShortOfTheQuote)?;

    assert!(
        *probe.cost > 0,
        "a stabilize must cost something or a pool below it cannot exist",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(false),
        "the wire reports the greyed button the panel is showing for a pool one short of the \
         sim's own stabilize quote: {probe:?}",
    );
    Ok(())
}

#[test]
fn a_pool_that_covers_the_stabilize_quote_is_offered_as_pressable() -> TestResult {
    let probe = stabilize_offer(Pool::TheQuote)?;

    assert!(
        *probe.cost > 0,
        "a stabilize must cost something or this case cannot tell a greyed button from a live one",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(true),
        "a pool that covers the quote leaves the stabilize button pressable: {probe:?}",
    );
    Ok(())
}
