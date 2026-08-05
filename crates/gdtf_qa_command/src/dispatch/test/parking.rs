use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::{command::CommandOutcome, message::QaResponse};

use crate::{
    dispatch::{CommandResponder, DeferredDelivery, DeferredReplies},
    test_support::{FakeSettle, FakeSettleCount, FakeSettleReply},
};

fn a_reply(waited: u32) -> FakeSettleReply {
    FakeSettleReply {
        waited: FakeSettleCount::new(waited),
    }
}

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
        ron::de::from_str::<FakeSettleReply>(reply.as_str()).ok(),
        Some(a_reply(1))
    );
    assert!(
        second.try_recv().is_err(),
        "the younger waiter must still be parked"
    );
}

#[test]
fn answer_next_on_an_empty_parking_reports_empty() {
    let mut parked = DeferredReplies::<FakeSettle>::default();

    assert_eq!(parked.answer_next(&a_reply(0)), DeferredDelivery::Empty);
    assert!(parked.is_empty());
}
