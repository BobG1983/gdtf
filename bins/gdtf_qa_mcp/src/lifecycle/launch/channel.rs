//! Env var names that enable and configure a host's net QA channel.

use gdtf_qa_protocol::ports::EDITOR_QA_PORT_VAR;

use super::values::EnvVarName;

const GAME_ENABLE_VAR: &str = "GDTF_NET_QA";

const GAME_PORT_VAR: &str = "GDTF_NET_QA_PORT";

const EDITOR_ENABLE_VAR: &str = "GDTF_EDITOR_NET_QA";

/// Enable flag and port env var names for a host.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QaChannel {
    enable: EnvVarName,
    port:   EnvVarName,
}

impl QaChannel {
    /// From explicit env names.
    #[must_use]
    pub const fn new(enable: EnvVarName, port: EnvVarName) -> Self {
        Self { enable, port }
    }

    /// Env var that enables the channel when set to `1`.
    #[must_use]
    pub const fn enable(&self) -> &EnvVarName {
        &self.enable
    }

    /// Env var that carries the listen port.
    #[must_use]
    pub const fn port(&self) -> &EnvVarName {
        &self.port
    }

    /// Game channel names.
    #[must_use]
    pub fn game() -> Self {
        Self::new(
            EnvVarName::new(GAME_ENABLE_VAR.to_owned()),
            EnvVarName::new(GAME_PORT_VAR.to_owned()),
        )
    }

    /// Editor channel names.
    #[must_use]
    pub fn editor() -> Self {
        Self::new(
            EnvVarName::new(EDITOR_ENABLE_VAR.to_owned()),
            EnvVarName::new(EDITOR_QA_PORT_VAR.to_owned()),
        )
    }
}

#[cfg(test)]
mod test {
    use gdtf_qa_protocol::ports::EDITOR_QA_PORT_VAR;

    use super::QaChannel;

    #[test]
    fn the_two_channels_share_no_variable_name() {
        let game = QaChannel::game();
        let editor = QaChannel::editor();
        assert_ne!(game.enable(), editor.enable());
        assert_ne!(game.port(), editor.port());
        assert_eq!(game.enable().as_str(), "GDTF_NET_QA");
        assert_eq!(editor.enable().as_str(), "GDTF_EDITOR_NET_QA");
        assert_eq!(editor.port().as_str(), "GDTF_EDITOR_NET_QA_PORT");
        assert_eq!(editor.port().as_str(), EDITOR_QA_PORT_VAR);
    }
}
