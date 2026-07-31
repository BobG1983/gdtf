//! Parking and answering, one call at a time.
//!
//! `answer_all` is what the fake settle handler uses, so the suite covers it end to end.
//! `answer_next` is the other half of the parking surface — a command that settles its
//! oldest waiter without releasing the rest — and nothing else in the crate calls it, so it
//! is exercised here.

use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::{command::CommandOutcome, envelope::QaResponse};

use crate::{
    dispatch::{CommandResponder, DeferredDelivery, DeferredReplies},
    test_support::{FakeSettle, FakeSettleCount, FakeSettleReply},
};

/// The reply a settled call carries.
fn a_reply(waited: u32) -> FakeSettleReply {
    FakeSettleReply {
        waited: FakeSettleCount::new(waited),
    }
}

/// `answer_next` releases the OLDEST waiter and leaves the rest parked with their own
/// deadlines intact.
#[test]
fn answer_next_releases_the_oldest_waiter_only() {
    let mut parked = DeferredReplies::<FakeSettle>::default();
    let (first_raw, first) = Responder::channel();
    let (second_raw, second) = Responder::channel();
    parked.park(CommandResponder::new(first_raw));
    parked.park(CommandResponder::new(second_raw));
    assert_eq!(parked.len(), 2);

    assert_eq!(
        parked.answer_next(&a_reply(1)),
        DeferredDelivery::Delivered,
        "a parked call was waiting"
    );
    assert_eq!(parked.len(), 1, "only the oldest was released");

    let Ok(QaResponse::Outcome(CommandOutcome::Ran { reply, .. })) = first.try_recv() else {
        unreachable!("the oldest waiter must have been answered");
    };
    assert_eq!(
        serde_json::from_str::<FakeSettleReply>(reply.as_str()).ok(),
        Some(a_reply(1))
    );
    assert!(
        second.try_recv().is_err(),
        "the younger waiter must still be parked"
    );
}

/// `answer_next` on nothing reports `Empty` rather than pretending it delivered.
#[test]
fn answer_next_on_an_empty_parking_reports_empty() {
    let mut parked = DeferredReplies::<FakeSettle>::default();

    assert_eq!(parked.answer_next(&a_reply(0)), DeferredDelivery::Empty);
    assert!(parked.is_empty());
}
