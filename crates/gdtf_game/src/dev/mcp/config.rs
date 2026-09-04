//! Game MCP identity and hello facts.

use cobalt_mcp_protocol::{
    message::{HelloFacts, ProtocolVersion, ServerNameNet},
    ports::McpPort,
};
use gdtf_mcp_ports::GAME_QA_PORT;

crate::support_item! {
    /// The QA protocol version this game speaks.
    const MCP_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::CURRENT;
}

crate::support_item! {
    /// The name the game announces itself under on the QA channel.
    const SERVER_NAME: &str = "gdtf-mcp";
}

pub(super) const DEFAULT_PORT: McpPort = McpPort::new(GAME_QA_PORT);

crate::support_item! {
    /// The identity the game sends in its QA hello.
    #[must_use]
    fn hello_facts() -> HelloFacts {
        HelloFacts::new(
            MCP_PROTOCOL_VERSION,
            ServerNameNet::new(SERVER_NAME.to_owned()),
        )
    }
}
