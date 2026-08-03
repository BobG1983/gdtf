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

    app.update();
    no_answer_yet(&channel);
    assert_eq!(
        app.world().resource::<DeferredReplies<FakeSettle>>().len(),
        1,
        "the handler parked the call instead of answering it"
    );

    app.update();
    app.update();
    no_answer_yet(&channel);

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

    app.update();

    assert_eq!(answer(&channel), QaResponse::Error(QaError::Timeout));
    assert!(
        app.world()
            .resource::<DeferredReplies<FakeSettle>>()
            .is_empty(),
        "the expired entry is dropped, not left to answer twice"
    );
}

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

#[test]
fn the_deferral_budget_expires_before_the_socket_could() {
    assert!(
        *DEFERRED_BUDGET < *DEFAULT_IO_TIMEOUT,
        "the deferral budget ({:?}) must expire strictly before the socket timeout ({:?})",
        *DEFERRED_BUDGET,
        *DEFAULT_IO_TIMEOUT
    );
}
