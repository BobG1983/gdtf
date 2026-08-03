use gdtf_net_qa_transport::NetQaPort;

use super::config::DEFAULT_EDITOR_PORT;

const EDITOR_NET_QA_ENV: &str = "GDTF_EDITOR_NET_QA";

const EDITOR_NET_QA_PORT_ENV: &str = "GDTF_EDITOR_NET_QA_PORT";

#[must_use]
fn recognised_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[must_use]
pub(super) fn editor_net_qa_enabled() -> bool {
    std::env::var(EDITOR_NET_QA_ENV).is_ok_and(|value| recognised_truthy(&value))
}

#[must_use]
pub(super) fn editor_port_from_env() -> NetQaPort {
    std::env::var(EDITOR_NET_QA_PORT_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
        .map_or(DEFAULT_EDITOR_PORT, NetQaPort::new)
}

#[cfg(test)]
mod test {
    use super::{DEFAULT_EDITOR_PORT, recognised_truthy};

            #[test]
    fn recognised_truthy_matches_the_four_affirmatives() {
        for yes in ["1", "true", "TRUE", " yes ", "On"] {
            assert!(recognised_truthy(yes), "{yes:?} should be truthy");
        }
        for no in ["", "0", "false", "off", "nope", "2"] {
            assert!(!recognised_truthy(no), "{no:?} should be falsey");
        }
    }

            #[test]
    fn default_editor_port_is_a_distinct_high_port() {
        assert!(*DEFAULT_EDITOR_PORT >= 1024);
        assert_ne!(*DEFAULT_EDITOR_PORT, 7616, "must not reuse the game's port");
    }
}
