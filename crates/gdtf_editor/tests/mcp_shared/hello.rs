use cobalt_mcp_protocol::message::{ProtocolVersion, QaResponse};
use gdtf_editor::EDITOR_QA_SERVER_NAME;

/// The handshake facts the editor answers a matching client version with.
pub(crate) fn assert_hello_ok(reply: &QaResponse) {
    assert!(
        matches!(
            reply,
            QaResponse::HelloOk(facts)
                if facts.protocol == ProtocolVersion::CURRENT
                    && *facts.server == EDITOR_QA_SERVER_NAME
        ),
        "expected the editor's HelloOk handshake facts, got {reply:?}",
    );
}
