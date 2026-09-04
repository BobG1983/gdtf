use cobalt_mcp_protocol::message::{McpResponse, ProtocolVersion};
use gdtf_editor::EDITOR_MCP_SERVER_NAME;

/// The handshake facts the editor answers a matching client version with.
pub(crate) fn assert_hello_ok(reply: &McpResponse) {
    assert!(
        matches!(
            reply,
            McpResponse::HelloOk(facts)
                if facts.protocol == ProtocolVersion::CURRENT
                    && *facts.server == EDITOR_MCP_SERVER_NAME
        ),
        "expected the editor's HelloOk handshake facts, got {reply:?}",
    );
}
