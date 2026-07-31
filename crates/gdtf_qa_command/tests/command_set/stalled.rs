//! A claimed call nobody drains is answered by the pending queue's frame deadline, not
//! left to hang the client's socket.
//!
//! This is the `sweep_pending::<CommandCall<C>>` registration in `register_command`. Delete
//! that one line and a command whose handler early-outs forever silently swallows every
//! call: the entry sits in `PendingQueue<CommandCall<C>>` with nobody to reap it and the
//! client waits until its own socket gives up.

use gdtf_qa_command::{
    command::QaCommand,
    test_support::{
        FAKE_COMMANDS_STALLED, FakeStall, fake_app, fake_facts_loaded, run_fake_command,
    },
};
use gdtf_qa_protocol::envelope::{QaError, QaResponse};

use crate::support::{answer, args, no_answer_yet, plain};

/// How many frames the pending queue's deadline needs to reap an unclaimed entry.
///
/// The budget is four frames of grace, ticked once per frame by the sweep in `Last`; the
/// entry is pushed during the first frame's `Update`, so the fifth frame's sweep is the one
/// that finds it spent. A change to the transport's budget shows up here as a failure of
/// the "answered by frame five, not before" pair below, which is the point.
const FRAMES_TO_EXPIRY: usize = 5;

/// A stalled call is answered `Timeout`, and not one frame earlier than the budget allows.
#[test]
fn a_call_no_handler_drains_is_answered_by_the_pending_deadline() {
    let mut app = fake_app(FAKE_COMMANDS_STALLED, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS_STALLED,
        &FakeStall::NAME,
        &args("{\"label\":\"nobody is listening\"}"),
        &plain(),
    );

    // Admitted and decoded on the first frame — the refusal paths are not what is under
    // test here, so the call must genuinely be in the queue.
    for frame in 1..FRAMES_TO_EXPIRY {
        app.update();
        assert!(
            channel.try_recv().is_err(),
            "the deadline must not fire early: it answered on frame {frame}"
        );
    }

    app.update();
    assert_eq!(
        answer(&channel),
        QaResponse::Error(QaError::Timeout),
        "an unclaimed call is answered by the pending queue's deadline sweep"
    );
}

/// The call really does reach the queue first — it is not refused at admission and then
/// mistaken for a timeout.
#[test]
fn the_stalled_call_is_admitted_before_it_expires() {
    let mut app = fake_app(FAKE_COMMANDS_STALLED, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS_STALLED,
        &FakeStall::NAME,
        &args("{\"label\":\"queued and abandoned\"}"),
        &plain(),
    );

    app.update();
    no_answer_yet(&channel);
}
