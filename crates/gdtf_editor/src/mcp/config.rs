//! The editor's MCP identity, its hello facts, and the resolution of the
//! port its listener binds.

use cobalt_mcp_protocol::{
    message::{HelloFacts, ProtocolVersion, ServerNameNet},
    ports::McpPort,
};

pub(super) const EDITOR_MCP_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::CURRENT;

/// Server name advertised on the editor MCP hello.
pub const EDITOR_MCP_SERVER_NAME: &str = "gdtf-editor-mcp";

// Env var carrying the port the editor's listener binds. No value, no listener.
const EDITOR_MCP_PORT_VAR: &str = "EDITOR_MCP_PORT";

#[must_use]
pub(super) fn editor_host_name() -> ServerNameNet {
    ServerNameNet::new(EDITOR_MCP_SERVER_NAME.to_owned())
}

#[must_use]
pub(super) fn editor_hello_facts() -> HelloFacts {
    HelloFacts::new(EDITOR_MCP_PROTOCOL_VERSION, editor_host_name())
}

#[must_use]
pub(super) fn editor_port_from(raw: Option<&str>) -> Option<McpPort> {
    raw.and_then(|value| value.trim().parse::<u16>().ok())
        .map(McpPort::new)
}

#[must_use]
pub(super) fn editor_port_from_env() -> Option<McpPort> {
    editor_port_from(std::env::var(EDITOR_MCP_PORT_VAR).ok().as_deref())
}

#[cfg(test)]
mod test {
    use cobalt_mcp_protocol::ports::McpPort;

    use super::editor_port_from;

    #[test]
    fn a_parsable_value_is_the_listen_port() {
        assert_eq!(editor_port_from(Some("7620")), Some(McpPort::new(7620)));
    }

    #[test]
    fn an_absent_value_opens_no_listener() {
        assert_eq!(editor_port_from(None), None);
    }

    #[test]
    fn an_unparsable_value_opens_no_listener() {
        assert_eq!(editor_port_from(Some("banana")), None);
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_parsing() {
        assert_eq!(editor_port_from(Some(" 7620 ")), Some(McpPort::new(7620)));
    }
}
