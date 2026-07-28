//! [`QaChannel`] — the two environment variables that switch a launched child's QA
//! control channel on and pick its port (GTW-808).
//!
//! The game and the editor are separate binaries with separate variable names, and that
//! difference is the whole reason a second launch target needs more than a different
//! package name: setting `GDTF_NET_QA` on an editor child leaves the editor inert, and
//! pointing both hosts at one variable would switch them both on at one port. Carrying
//! the pair in the launch recipe keeps the spawner free of any per-host branch — it sets
//! whatever names the recipe names.

use super::values::EnvVarName;

/// The game's channel switch — read by `crates/gdtf_app/src/dev/net_qa/env.rs`.
const GAME_ENABLE_VAR: &str = "GDTF_NET_QA";

/// The game's port variable — read by the same module.
const GAME_PORT_VAR: &str = "GDTF_NET_QA_PORT";

/// The editor's channel switch — read by
/// `crates/gdtf_content_editor/src/net_qa/env.rs`.
const EDITOR_ENABLE_VAR: &str = "GDTF_EDITOR_NET_QA";

/// The editor's port variable — read by the same module.
const EDITOR_PORT_VAR: &str = "GDTF_EDITOR_NET_QA_PORT";

/// The pair of environment-variable NAMES one host's QA control channel is driven by.
///
/// A plain aggregate of two typed names (not a newtype — it has two fields). The
/// launcher sets `enable` to `1` and `port` to the port it is about to probe, LAST, so a
/// recipe's own overrides can never displace either.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QaChannel {
    /// The variable that opts the child build into its QA control channel.
    enable: EnvVarName,
    /// The variable that picks the child's loopback listen port.
    port:   EnvVarName,
}

impl QaChannel {
    /// Build a channel from its two variable names.
    #[must_use]
    pub const fn new(enable: EnvVarName, port: EnvVarName) -> Self {
        Self { enable, port }
    }

    /// The variable that opts the child into its QA control channel.
    #[must_use]
    pub const fn enable(&self) -> &EnvVarName {
        &self.enable
    }

    /// The variable that picks the child's loopback listen port.
    #[must_use]
    pub const fn port(&self) -> &EnvVarName {
        &self.port
    }

    /// The GAME's channel: `GDTF_NET_QA` / `GDTF_NET_QA_PORT`.
    #[must_use]
    pub fn game() -> Self {
        Self::new(
            EnvVarName::new(GAME_ENABLE_VAR.to_owned()),
            EnvVarName::new(GAME_PORT_VAR.to_owned()),
        )
    }

    /// The EDITOR's channel: `GDTF_EDITOR_NET_QA` / `GDTF_EDITOR_NET_QA_PORT`.
    #[must_use]
    pub fn editor() -> Self {
        Self::new(
            EnvVarName::new(EDITOR_ENABLE_VAR.to_owned()),
            EnvVarName::new(EDITOR_PORT_VAR.to_owned()),
        )
    }
}

#[cfg(test)]
mod test {
    use super::QaChannel;

    /// The two hosts' channels share no variable name — a single shared pair would switch
    /// both binaries on and point them at one port.
    #[test]
    fn the_two_channels_share_no_variable_name() {
        let game = QaChannel::game();
        let editor = QaChannel::editor();
        assert_ne!(game.enable(), editor.enable());
        assert_ne!(game.port(), editor.port());
        assert_eq!(game.enable().as_str(), "GDTF_NET_QA");
        assert_eq!(editor.enable().as_str(), "GDTF_EDITOR_NET_QA");
        assert_eq!(editor.port().as_str(), "GDTF_EDITOR_NET_QA_PORT");
    }
}
