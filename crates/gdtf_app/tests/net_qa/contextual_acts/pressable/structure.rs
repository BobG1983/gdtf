//! The door and emplacement acts grey out when the pool is short of their own quote.

use gdtf_app::qa_wire::offer::{ContextualActNet, OfferPressableNet};
use gdtf_battle_sim::acts::{
    enter_emplacement_tu_cost, exit_emplacement_tu_cost, open_door_tu_cost,
};

use super::probe::{OfferAtPool, Pool, from_tuning, offer_at_pool};
use crate::{
    contextual_acts::{
        door::door_beside_the_shooter,
        emplacement::{emplacement_beside_the_shooter, manned_emplacement_under_the_shooter},
    },
    socket_support::{TestError, TestResult},
};

/// The open-door offer, read with the shooter's pool set from `pool`.
fn open_door_offer(pool: Pool) -> Result<OfferAtPool, TestError> {
    offer_at_pool(
        ContextualActNet::OpenDoor,
        door_beside_the_shooter,
        from_tuning(open_door_tu_cost),
        pool,
    )
}

/// The mount offer, read with the shooter's pool set from `pool`.
fn enter_offer(pool: Pool) -> Result<OfferAtPool, TestError> {
    offer_at_pool(
        ContextualActNet::EnterEmplacement,
        emplacement_beside_the_shooter,
        from_tuning(enter_emplacement_tu_cost),
        pool,
    )
}

/// The dismount offer, read with the shooter already manning a seat and its pool set from `pool`.
fn exit_offer(pool: Pool) -> Result<OfferAtPool, TestError> {
    offer_at_pool(
        ContextualActNet::ExitEmplacement,
        manned_emplacement_under_the_shooter,
        from_tuning(exit_emplacement_tu_cost),
        pool,
    )
}

#[test]
fn a_pool_below_the_open_door_quote_is_offered_as_not_pressable() -> TestResult {
    let probe = open_door_offer(Pool::ShortOfTheQuote)?;

    assert!(
        *probe.cost > 0,
        "opening a door must cost something or a pool below it cannot exist",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(false),
        "the wire reports the greyed button the panel is showing for a pool one short of the \
         sim's own door quote: {probe:?}",
    );
    Ok(())
}

#[test]
fn a_pool_that_covers_the_open_door_quote_is_offered_as_pressable() -> TestResult {
    let probe = open_door_offer(Pool::TheQuote)?;

    assert!(
        *probe.cost > 0,
        "opening a door must cost something or this case cannot tell a greyed button from a live \
         one",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(true),
        "a pool that covers the quote leaves the door button pressable: {probe:?}",
    );
    Ok(())
}

#[test]
fn a_pool_below_the_enter_emplacement_quote_is_offered_as_not_pressable() -> TestResult {
    let probe = enter_offer(Pool::ShortOfTheQuote)?;

    assert!(
        *probe.cost > 0,
        "mounting an emplacement must cost something or a pool below it cannot exist",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(false),
        "the wire reports the greyed button the panel is showing for a pool one short of the \
         sim's own mount quote: {probe:?}",
    );
    Ok(())
}

#[test]
fn a_pool_that_covers_the_enter_emplacement_quote_is_offered_as_pressable() -> TestResult {
    let probe = enter_offer(Pool::TheQuote)?;

    assert!(
        *probe.cost > 0,
        "mounting an emplacement must cost something or this case cannot tell a greyed button \
         from a live one",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(true),
        "a pool that covers the quote leaves the mount button pressable: {probe:?}",
    );
    Ok(())
}

#[test]
fn a_pool_below_the_exit_emplacement_quote_is_offered_as_not_pressable() -> TestResult {
    let probe = exit_offer(Pool::ShortOfTheQuote)?;

    assert!(
        *probe.cost > 0,
        "dismounting must cost something or a pool below it cannot exist",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(false),
        "the wire reports the greyed button the panel is showing for a pool one short of the \
         sim's own dismount quote: {probe:?}",
    );
    Ok(())
}

#[test]
fn a_pool_that_covers_the_exit_emplacement_quote_is_offered_as_pressable() -> TestResult {
    let probe = exit_offer(Pool::TheQuote)?;

    assert!(
        *probe.cost > 0,
        "dismounting must cost something or this case cannot tell a greyed button from a live one",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(true),
        "a pool that covers the quote leaves the dismount button pressable: {probe:?}",
    );
    Ok(())
}
