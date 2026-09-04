use core::time::Duration;

use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{DEFERRED_BUDGET, DeferredBudget, DeferredReplies},
    test_support::{
        FAKE_COMMANDS, FAKE_COMMANDS_GROWN, FAKE_COMMANDS_STALLED, FakePhase, FakeSettle,
        FakeSettleReply, FakeSettleSignal, fake_app, fake_facts_loaded, run_fake_command,
    },
};
use cobalt_mcp_protocol::{
    command::CommandName,
    message::{QaError, QaResponse},
    timeouts::DEFAULT_REPLY_TIMEOUT,
};

use crate::support::{answer, args, no_answer_yet, plain, ran};

#[test]
fn a_parked_reply_settles_on_a_later_frame() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakeSettle::NAME,
        &args("()"),
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
        &args("()"),
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
fn a_command_that_declares_no_budget_parks_against_the_default() {
    let app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    assert_eq!(
        app.world()
            .resource::<DeferredReplies<FakePhase>>()
            .budget(),
        DEFERRED_BUDGET,
        "a command that declares nothing must be registered with the crate default"
    );
}

#[test]
fn a_command_that_declares_a_budget_is_registered_with_its_own() {
    let app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let registered = app
        .world()
        .resource::<DeferredReplies<FakeSettle>>()
        .budget();
    assert_eq!(
        registered,
        FakeSettle::DEFERRED_BUDGET,
        "registration must apply the command's own declared budget"
    );
    assert_ne!(
        registered, DEFERRED_BUDGET,
        "this case proves nothing unless the declared budget differs from the default"
    );
    assert!(
        *registered < *DEFAULT_REPLY_TIMEOUT,
        "a declared budget ({:?}) must still expire before the socket stops waiting for the reply \
         ({:?})",
        *registered,
        *DEFAULT_REPLY_TIMEOUT
    );
}

#[test]
fn the_erased_view_reports_each_command_its_own_budget() {
    let budget_of = |name: &CommandName| {
        FAKE_COMMANDS_GROWN
            .iter()
            .find(|command| command.name() == *name)
            .map(|command| command.deferred_budget())
    };

    assert_eq!(
        budget_of(&FakeSettle::NAME),
        Some(FakeSettle::DEFERRED_BUDGET),
        "the erased view must publish the command's own declared budget, not a crate constant"
    );
    assert_ne!(
        budget_of(&FakeSettle::NAME),
        Some(DEFERRED_BUDGET),
        "this case proves nothing unless the declared budget differs from the default"
    );
    assert_eq!(
        budget_of(&FakePhase::NAME),
        Some(DEFERRED_BUDGET),
        "a command that declares nothing must publish the crate default"
    );
}

#[test]
fn every_published_command_expires_before_the_socket_could() {
    for set in [FAKE_COMMANDS_GROWN, FAKE_COMMANDS_STALLED] {
        for command in set {
            let budget = command.deferred_budget();
            assert!(
                *budget < *DEFAULT_REPLY_TIMEOUT,
                "`{}`'s deferral budget ({:?}) must expire strictly before the socket stops \
                 waiting for the reply ({:?})",
                command.name().as_str(),
                *budget,
                *DEFAULT_REPLY_TIMEOUT
            );
        }
    }
}
