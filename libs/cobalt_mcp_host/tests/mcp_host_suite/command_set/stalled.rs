use cobalt_mcp_host::{
    command::McpCommand,
    test_support::{
        FAKE_COMMANDS_STALLED, FakeStall, fake_app, fake_facts_loaded, run_fake_command,
    },
};
use cobalt_mcp_protocol::message::{McpResponse, McpSessionError};

use crate::command_set::support::{answer, args, no_answer_yet, plain};

const FRAMES_TO_EXPIRY: usize = 5;

#[test]
fn a_call_no_handler_drains_is_answered_by_the_pending_deadline() {
    let mut app = fake_app(FAKE_COMMANDS_STALLED, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS_STALLED,
        &FakeStall::NAME,
        &args("(label:\"nobody is listening\")"),
        &plain(),
    );

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
        McpResponse::Error(McpSessionError::Timeout),
        "an unclaimed call is answered by the pending queue's deadline sweep"
    );
}

#[test]
fn the_stalled_call_is_admitted_before_it_expires() {
    let mut app = fake_app(FAKE_COMMANDS_STALLED, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS_STALLED,
        &FakeStall::NAME,
        &args("(label:\"queued and abandoned\")"),
        &plain(),
    );

    app.update();
    no_answer_yet(&channel);
}
