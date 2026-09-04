//! The editor's MCP identity, its hello facts, and the resolution of the
//! port its listener binds.

use cobalt_mcp_protocol::{
    message::{HelloFacts, ProtocolVersion, ServerNameNet},
    ports::McpPort,
};
use gdtf_mcp_ports::{EDITOR_QA_PORT, EDITOR_QA_PORT_VAR};

pub(super) const EDITOR_QA_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::CURRENT;

/// Server name advertised on the editor QA hello.
pub const EDITOR_QA_SERVER_NAME: &str = "gdtf-editor-mcp";

pub(super) const DEFAULT_EDITOR_PORT: McpPort = McpPort::new(EDITOR_QA_PORT);

#[must_use]
pub(super) fn editor_host_name() -> ServerNameNet {
    ServerNameNet::new(EDITOR_QA_SERVER_NAME.to_owned())
}

#[must_use]
pub(super) fn editor_hello_facts() -> HelloFacts {
    HelloFacts::new(EDITOR_QA_PROTOCOL_VERSION, editor_host_name())
}

#[must_use]
pub(super) fn editor_port_from(raw: Option<&str>) -> McpPort {
    raw.and_then(|value| value.trim().parse::<u16>().ok())
        .map_or(DEFAULT_EDITOR_PORT, McpPort::new)
}

#[must_use]
pub(super) fn editor_port_from_env() -> McpPort {
    editor_port_from(std::env::var(EDITOR_QA_PORT_VAR).ok().as_deref())
}

#[cfg(test)]
mod test {
    use cobalt_mcp_protocol::ports::McpPort;
    use gdtf_mcp_ports::GAME_QA_PORT;

    use super::{DEFAULT_EDITOR_PORT, editor_port_from};

    #[test]
    fn default_editor_port_is_a_distinct_high_port() {
        assert!(*DEFAULT_EDITOR_PORT >= 1024);
        assert_ne!(
            *DEFAULT_EDITOR_PORT, GAME_QA_PORT,
            "must not reuse the game's port"
        );
    }

    #[test]
    fn a_parsable_value_is_the_listen_port() {
        assert_eq!(editor_port_from(Some("7620")), McpPort::new(7620));
    }

    #[test]
    fn an_absent_value_falls_back_to_the_default() {
        assert_eq!(editor_port_from(None), DEFAULT_EDITOR_PORT);
    }

    #[test]
    fn an_unparsable_value_falls_back_to_the_default() {
        assert_eq!(editor_port_from(Some("banana")), DEFAULT_EDITOR_PORT);
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_parsing() {
        assert_eq!(editor_port_from(Some(" 7620 ")), McpPort::new(7620));
    }
}
