use cobalt_mcp_protocol::message::{McpRequest, McpResponse, ProtocolVersion};
use gdtf_editor::EditorState;

use crate::{
    harness::{editor_app_listening, editor_state},
    hello::assert_hello_ok,
    socket::Client,
    support::TestError,
};

/// Send `request` before the editor has run a single frame, and hand back what it answered.
pub(crate) fn reply_answered_during_load(
    request: McpRequest,
    what: &str,
) -> Result<McpResponse, TestError> {
    let (mut app, port) = editor_app_listening()?;
    let mut client = Client::connect(port)?;
    client.send(&McpRequest::Hello(ProtocolVersion::CURRENT))?;
    client.send(&request)?;
    assert_eq!(
        editor_state(&app),
        Some(EditorState::Load),
        "the app has run no frames yet, so the editor's own state machine must still be in its \
         default Load pass when {what} goes out",
    );

    let negotiated = client.read(&mut app)?;
    assert_hello_ok(&negotiated);
    client.read(&mut app)
}
