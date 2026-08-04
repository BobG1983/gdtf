//! Editor net QA identity and hello facts.

use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{
    message::{HelloFacts, ProtocolVersion, ServerNameNet},
    ports::EDITOR_QA_PORT,
};

pub(super) const EDITOR_QA_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::CURRENT;

/// Server name advertised on the editor QA hello.
pub const EDITOR_QA_SERVER_NAME: &str = "gdtf-editor-net-qa";

pub(super) const DEFAULT_EDITOR_PORT: NetQaPort = NetQaPort::new(EDITOR_QA_PORT);

#[must_use]
pub(super) fn editor_hello_facts() -> HelloFacts {
    HelloFacts::new(
        EDITOR_QA_PROTOCOL_VERSION,
        ServerNameNet::new(EDITOR_QA_SERVER_NAME.to_owned()),
    )
}
