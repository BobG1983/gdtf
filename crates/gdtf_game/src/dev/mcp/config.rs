//! Game MCP identity and hello facts.

use cobalt_mcp_protocol::message::{HelloFacts, ProtocolVersion, ServerNameNet};

crate::support_item! {
    /// The MCP protocol version this game speaks.
    const MCP_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::CURRENT;
}

crate::support_item! {
    /// The name the game announces itself under on the MCP channel.
    const SERVER_NAME: &str = "gdtf-mcp";
}

crate::support_item! {
    /// The identity the game sends in its MCP hello.
    #[must_use]
    fn hello_facts() -> HelloFacts {
        HelloFacts::new(
            MCP_PROTOCOL_VERSION,
            ServerNameNet::new(SERVER_NAME.to_owned()),
        )
    }
}
