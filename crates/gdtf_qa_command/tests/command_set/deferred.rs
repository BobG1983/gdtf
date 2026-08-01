//! A deferred reply settles on a later frame — and one that never settles expires with an
//! answer, strictly before the transport's socket timeout could fire.

use core::time::Duration;

use gdtf_net_qa_transport::DEFAULT_IO_TIMEOUT;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{DEFERRED_BUDGET, DeferredBudget, DeferredReplies},
    test_support::{
        FAKE_COMMANDS, FakeSettle, FakeSettleReply, FakeSettleSignal, fake_app, fake_facts_loaded,
        run_fake_command,
    },
};
use gdtf_qa_protocol::message::{QaError, QaResponse};

use crate::support::{answer, args, no_answer_yet, plain, ran};

/// A call parked in `DeferredReplies<C>` is delivered on the frame its condition is met.
#[test]
fn a_parked_reply_settles_on_a_later_frame() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakeSettle::NAME,
        &args("{}"),
        &plain(),
    );

    // Frame 1: claimed, then parked rather than answered.
    app.update();
    no_answer_yet(&channel);
    assert_eq!(
        app.world().resource::<DeferredReplies<FakeSettle>>().len(),
        1,
        "the handler parked the call instead of answering it"
    );

    // Frames 2 and 3: still parked, still unanswered.
    app.update();
    app.update();
    no_answer_yet(&channel);

    // The world condition arrives.
    app.world_mut().resource_mut::<FakeSettleSignal>().raise();
    app.update();

    let reply: FakeSettleReply = ran(&channel);
    assert_eq!(
        *reply.waited, 1,
        "the reply carries what the settling frame knew"
    );
    assert!(
        app.world()
            .resource::<DeferredReplies<FakeSettle>>()
            .is_empty(),
        "a delivered reply is no longer parked"
    );
}

/// A parked reply that never settles is swept with a deadline answer.
///
/// The budget is driven to zero so expiry is deterministic — the alternative is sleeping
/// through the real budget, which would make the suite slow and flaky for no extra
/// coverage. The real DEFAULT is pinned by the inequality test below.
#[test]
fn a_reply_that_never_settles_is_swept_with_a_deadline_answer() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    app.world_mut()
        .resource_mut::<DeferredReplies<FakeSettle>>()
        .set_budget(DeferredBudget::new(Duration::ZERO));

    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakeSettle::NAME,
        &args("{}"),
        &plain(),
    );

    // One frame: claimed, parked, and swept in `Last` — the signal is never raised.
    app.update();

    assert_eq!(answer(&channel), QaResponse::Error(QaError::Timeout));
    assert!(
        app.world()
            .resource::<DeferredReplies<FakeSettle>>()
            .is_empty(),
        "the expired entry is dropped, not left to answer twice"
    );
}

/// A freshly registered command's parking carries the DEFAULT budget.
///
/// Without this the inequality below is pinned in the abstract while
/// `impl Default for DeferredReplies` hands every real host something else entirely — the
/// two tests above never see the default, because one drives the budget to zero and the
/// other never lets a deadline fire.
#[test]
fn a_registered_command_parks_against_the_default_budget() {
    let app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    assert_eq!(
        app.world()
            .resource::<DeferredReplies<FakeSettle>>()
            .budget(),
        DEFERRED_BUDGET,
        "registering a command must give its parking the default budget, which is the only \
         budget the inequality below covers"
    );
}

/// The deadline answer is guaranteed to reach a client that is still listening.
///
/// A deferral that outlived the socket would surface as a dead connection instead of an
/// answer, which is the failure this inequality exists to prevent.
#[test]
fn the_deferral_budget_expires_before_the_socket_could() {
    assert!(
        *DEFERRED_BUDGET < *DEFAULT_IO_TIMEOUT,
        "the deferral budget ({:?}) must expire strictly before the socket timeout ({:?})",
        *DEFERRED_BUDGET,
        *DEFAULT_IO_TIMEOUT
    );
}
