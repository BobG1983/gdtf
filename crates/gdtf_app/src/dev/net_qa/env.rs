//! The `GDTF_NET_QA` / `GDTF_NET_QA_PORT` environment gates (GTW-736).
//!
//! The env half of the plugin's activation gate (the `cfg!(debug_assertions)` +
//! `feature = "net_qa"` half lives at the [`crate::dev::plugin`] wiring site): a
//! `net_qa` build is INERT until `GDTF_NET_QA` is set truthy, and the listen port is
//! `GDTF_NET_QA_PORT` (port only — the interface is never configurable).

use super::config::{DEFAULT_PORT, NetQaPort};

/// The environment variable that opts a `net_qa` build into the QA control channel.
const NET_QA_ENV: &str = "GDTF_NET_QA";

/// The environment variable that picks the loopback listen PORT (the interface is not
/// configurable — it is always [`Ipv4Addr::LOCALHOST`](std::net::Ipv4Addr::LOCALHOST)).
const NET_QA_PORT_ENV: &str = "GDTF_NET_QA_PORT";

/// Whether `value` spells "enabled": `1` / `true` / `yes` / `on` after trimming,
/// case-insensitive; anything else is disabled.
///
/// Deliberately intra-module — the workspace already has another production truthy parser
/// (the presenter's `ReachableOverlayEnabled::from_env`), short of the rule of three, so
/// there is no shared cross-crate helper to reach for.
#[must_use]
fn recognised_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Whether the DEV `net_qa` QA control channel is enabled for this process.
///
/// Reads [`NET_QA_ENV`] (`GDTF_NET_QA`) and treats `1` / `true` / `yes` / `on`
/// (case-insensitive, trimmed) as enabled; anything else — including the variable being
/// unset or empty — is disabled. This is the env half of the gate; the
/// `cfg!(debug_assertions)` + `feature = "net_qa"` half lives at the
/// [`crate::dev::plugin`] wiring site, so a release build never even compiles the server
/// in.
#[must_use]
pub(super) fn net_qa_enabled() -> bool {
    std::env::var(NET_QA_ENV).is_ok_and(|value| recognised_truthy(&value))
}

/// The loopback port [`from_env`](super::plugin::NetQaPlugin::from_env) binds — the
/// parsed [`NET_QA_PORT_ENV`] (`GDTF_NET_QA_PORT`) value, or [`DEFAULT_PORT`] when the
/// variable is unset or does not parse as a `u16`.
///
/// Port ONLY — the interface is never read from the environment.
#[must_use]
pub(super) fn port_from_env() -> NetQaPort {
    std::env::var(NET_QA_PORT_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
        .map_or(DEFAULT_PORT, NetQaPort::new)
}

#[cfg(test)]
mod test {
    use super::{DEFAULT_PORT, recognised_truthy};

    /// The truthy parser recognises exactly the four affirmatives, trimmed +
    /// case-insensitive, and nothing else.
    #[test]
    fn recognised_truthy_matches_the_four_affirmatives() {
        for yes in ["1", "true", "TRUE", " yes ", "On"] {
            assert!(recognised_truthy(yes), "{yes:?} should be truthy");
        }
        for no in ["", "0", "false", "off", "nope", "2"] {
            assert!(!recognised_truthy(no), "{no:?} should be falsey");
        }
    }

    /// `DEFAULT_PORT` is a non-privileged, non-zero port (a real fallback, not the
    /// OS-ephemeral `0`).
    #[test]
    fn default_port_is_a_real_high_port() {
        assert!(*DEFAULT_PORT >= 1024);
    }
}
