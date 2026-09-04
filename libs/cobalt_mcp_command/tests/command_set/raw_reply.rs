use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, take_calls},
    test_support::{
        FAKE_COMMANDS_STALLED, FakeStall, fake_app, fake_facts_loaded, run_fake_command,
    },
};
use cobalt_mcp_protocol::message::{McpResponse, McpSessionError};
use cobalt_mcp_transport::PendingQueue;

use crate::support::{answer, args, plain};

#[test]
fn a_command_can_answer_with_a_raw_protocol_response() {
    let mut app = fake_app(FAKE_COMMANDS_STALLED, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS_STALLED,
        &FakeStall::NAME,
        &args("(label:\"answered by hand\")"),
        &plain(),
    );

    app.update();

    let mut queue = app
        .world_mut()
        .resource_mut::<PendingQueue<CommandCall<FakeStall>>>();
    let mut claimed = take_calls::<FakeStall>(&mut queue);
    let Some((_args, responder)) = claimed.pop() else {
        unreachable!("the claim system moved the call into the command's own queue");
    };

    responder
        .into_inner()
        .reply(McpResponse::Error(McpSessionError::Busy));

    assert_eq!(
        answer(&channel),
        McpResponse::Error(McpSessionError::Busy),
        "a raw response built outside the crate reaches the call's channel"
    );
}
