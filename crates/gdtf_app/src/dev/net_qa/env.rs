use gdtf_net_qa_transport::NetQaPort;

use super::config::DEFAULT_PORT;

/// Debug builds always listen; there is no env arming.
#[must_use]
pub(super) const fn net_qa_enabled() -> bool {
    true
}

/// Shared constant port from `gdtf_qa_protocol::ports`.
#[must_use]
pub(super) const fn port_from_env() -> NetQaPort {
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
