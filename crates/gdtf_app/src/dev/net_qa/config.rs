//! What stays here is THIS host's policy: the protocol version this server speaks, its
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::message::{HelloFacts, ProtocolVersion, ServerNameNet};

crate::support_item! {
                                                const NET_QA_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::CURRENT;
}

crate::support_item! {
                                const SERVER_NAME: &str = "gdtf-net-qa";
}

pub(super) const DEFAULT_PORT: NetQaPort = NetQaPort::new(7616);

crate::support_item! {
                                                #[must_use]
    fn hello_facts() -> HelloFacts {
        HelloFacts::new(
            NET_QA_PROTOCOL_VERSION,
            ServerNameNet::new(SERVER_NAME.to_owned()),
        )
    }
}
