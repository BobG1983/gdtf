use cobalt_mcp_protocol::ports::McpPort;

use super::config::DEFAULT_PORT;

/// A build with the `mcp` feature always listens; there is no env arming.
#[must_use]
pub(super) const fn mcp_enabled() -> bool {
    true
}

/// The fixed game port, `gdtf_mcp_ports::GAME_QA_PORT`, read from no environment.
#[must_use]
pub(super) const fn port_from_env() -> McpPort {
    DEFAULT_PORT
}

#[cfg(test)]
mod test {
    use super::DEFAULT_PORT;

    #[test]
    fn default_port_is_a_real_high_port() {
        assert!(*DEFAULT_PORT >= 1024);
    }
}
