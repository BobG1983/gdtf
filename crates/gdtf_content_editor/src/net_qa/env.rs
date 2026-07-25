//! The `GDTF_EDITOR_NET_QA` / `GDTF_EDITOR_NET_QA_PORT` environment gates (GTW-804).
//!
//! The env half of the plugin's activation gate (the `cfg!(debug_assertions)` +
//! `feature = "net_qa"` half lives at the `crate::app` wiring site): a `net_qa` editor build
//! is INERT until `GDTF_EDITOR_NET_QA` is set truthy, and the listen port is
//! `GDTF_EDITOR_NET_QA_PORT` (port only — the interface is never configurable).
//!
//! The names mirror the game's `GDTF_NET_QA` / `GDTF_NET_QA_PORT` convention with the
//! editor's own `GDTF_EDITOR_*` prefix, which the editor already uses for its other dev-only
//! affordances (`GDTF_EDITOR_SHOT` and friends). Sharing the game's variables would switch
//! both hosts on at once and point them at one port.

use gdtf_net_qa_transport::NetQaPort;

use super::config::DEFAULT_EDITOR_PORT;

/// The environment variable that opts a `net_qa` editor build into the QA control channel.
const EDITOR_NET_QA_ENV: &str = "GDTF_EDITOR_NET_QA";

/// The environment variable that picks the loopback listen PORT (the interface is not
/// configurable — it is always [`Ipv4Addr::LOCALHOST`](std::net::Ipv4Addr::LOCALHOST)).
const EDITOR_NET_QA_PORT_ENV: &str = "GDTF_EDITOR_NET_QA_PORT";

/// Whether `value` spells "enabled": `1` / `true` / `yes` / `on` after trimming,
/// case-insensitive; anything else is disabled.
///
/// Deliberately intra-module, matching the game's own intra-module copy: the two hosts are
/// separate crates with no shared env helper (GTW-803 left activation policy with each host),
/// and a truthy parser is three lines.
#[must_use]
fn recognised_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Whether the DEV `net_qa` QA control channel is enabled for this editor process.
///
/// Reads [`EDITOR_NET_QA_ENV`] (`GDTF_EDITOR_NET_QA`) and treats `1` / `true` / `yes` / `on`
/// (case-insensitive, trimmed) as enabled; anything else — including the variable being unset
/// or empty — is disabled. This is the env half of the gate; the `cfg!(debug_assertions)` +
/// `feature = "net_qa"` half lives at the `crate::app` wiring site, so a release editor
/// never even compiles the server in.
#[must_use]
pub(super) fn editor_net_qa_enabled() -> bool {
    std::env::var(EDITOR_NET_QA_ENV).is_ok_and(|value| recognised_truthy(&value))
}

/// The loopback port [`from_env`](super::NetQaEditorPlugin::from_env) binds — the parsed
/// [`EDITOR_NET_QA_PORT_ENV`] (`GDTF_EDITOR_NET_QA_PORT`) value, or [`DEFAULT_EDITOR_PORT`]
/// when the variable is unset or does not parse as a `u16`.
///
/// Port ONLY — the interface is never read from the environment.
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

    /// The editor's default port is a real high port AND is distinct from the game's `7616`
    /// (the two hosts must never contend for one socket).
    #[test]
    fn default_editor_port_is_a_distinct_high_port() {
        assert!(*DEFAULT_EDITOR_PORT >= 1024);
        assert_ne!(*DEFAULT_EDITOR_PORT, 7616, "must not reuse the game's port");
    }
}
