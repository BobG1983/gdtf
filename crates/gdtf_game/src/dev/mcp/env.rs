use cobalt_mcp_protocol::ports::McpPort;

// Env var carrying the port the game's listener binds. No value, no listener.
const GDTF_MCP_PORT_VAR: &str = "GDTF_MCP_PORT";

/// The listen port a raw value names, or `None` when it is absent or unparsable.
#[must_use]
pub(super) fn port_from(raw: Option<&str>) -> Option<McpPort> {
    raw.and_then(|value| value.trim().parse::<u16>().ok())
        .map(McpPort::new)
}

/// The listen port `GDTF_MCP_PORT` names, or `None` when it is unset.
#[must_use]
pub(super) fn port_from_env() -> Option<McpPort> {
    port_from(std::env::var(GDTF_MCP_PORT_VAR).ok().as_deref())
}

#[cfg(test)]
mod test {
    use cobalt_mcp_protocol::ports::McpPort;

    use super::port_from;

    #[test]
    fn a_parsable_value_is_the_listen_port() {
        assert_eq!(port_from(Some("7620")), Some(McpPort::new(7620)));
    }

    #[test]
    fn an_absent_value_opens_no_listener() {
        assert_eq!(port_from(None), None);
    }

    #[test]
    fn an_unparsable_value_opens_no_listener() {
        assert_eq!(port_from(Some("banana")), None);
    }
}
