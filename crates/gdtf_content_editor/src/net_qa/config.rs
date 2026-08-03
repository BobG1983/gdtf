//! The editor's own POLICY half of the activation gate: the protocol version it negotiates,
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::message::{HelloFacts, ProtocolVersion, ServerNameNet};

pub(super) const EDITOR_QA_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::CURRENT;

pub const EDITOR_QA_SERVER_NAME: &str = "gdtf-editor-net-qa";

pub(super) const DEFAULT_EDITOR_PORT: NetQaPort = NetQaPort::new(7617);

#[must_use]
pub(super) fn editor_hello_facts() -> HelloFacts {
    HelloFacts::new(
        EDITOR_QA_PROTOCOL_VERSION,
        ServerNameNet::new(EDITOR_QA_SERVER_NAME.to_owned()),
    )
}
