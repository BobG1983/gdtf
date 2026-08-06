//! Game net QA identity and hello facts.

use gdtf_qa_protocol::{
    message::{HelloFacts, ProtocolVersion, ServerNameNet},
    ports::{GAME_QA_PORT, NetQaPort},
};

crate::support_item! {
    /// The QA protocol version this game speaks.
    const NET_QA_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::CURRENT;
}

crate::support_item! {
    /// The name the game announces itself under on the QA channel.
    const SERVER_NAME: &str = "gdtf-net-qa";
}

pub(super) const DEFAULT_PORT: NetQaPort = NetQaPort::new(GAME_QA_PORT);

crate::support_item! {
    /// The identity the game sends in its QA hello.
    #[must_use]
    fn hello_facts() -> HelloFacts {
        HelloFacts::new(
            NET_QA_PROTOCOL_VERSION,
            ServerNameNet::new(SERVER_NAME.to_owned()),
        )
    }
}
