use gdtf_net_qa_transport::NetQaPort;

use super::config::DEFAULT_EDITOR_PORT;

/// Debug builds always listen. No env arming (GTW-969).
#[must_use]
pub(super) const fn editor_net_qa_enabled() -> bool {
    true
}

/// Shared constant port from `gdtf_qa_protocol::ports`.
#[must_use]
pub(super) const fn editor_port_from_env() -> NetQaPort {
    DEFAULT_EDITOR_PORT
}

#[cfg(test)]
mod test {
    use super::DEFAULT_EDITOR_PORT;
    use gdtf_qa_protocol::ports::GAME_QA_PORT;

    #[test]
    fn default_editor_port_is_a_distinct_high_port() {
        assert!(*DEFAULT_EDITOR_PORT >= 1024);
        assert_ne!(*DEFAULT_EDITOR_PORT, GAME_QA_PORT, "must not reuse the game's port");
    }
}
