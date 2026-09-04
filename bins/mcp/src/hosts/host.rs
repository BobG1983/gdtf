//! Which host the bridge is talking to.

use crate::{
    lifecycle::{LaunchSpec, LifecycleConfig},
    link::QaPort,
};

// The loopback ports the two hosts listen on. The bridge names only the wire protocol, so it
// keeps its own copy the way it already keeps the enable and port env var names.
const DEFAULT_GAME_PORT: QaPort = QaPort::new(7616);

const DEFAULT_EDITOR_PORT: QaPort = QaPort::new(7617);

/// Target process the MCP tools operate on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QaHost {
    /// Game binary.
    Game,
    /// Content editor binary.
    Editor,
}

impl QaHost {
    /// Both hosts.
    pub const ALL: [Self; 2] = [Self::Game, Self::Editor];

    /// Short label used in tool args (`game` / `editor`).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::Editor => "editor",
        }
    }

    /// Parse a label into a host.
    #[must_use]
    pub fn from_label(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|host| host.label() == word)
    }

    /// Human-readable stop tool name for error messages.
    #[must_use]
    pub const fn stop_tool_name(self) -> &'static str {
        match self {
            Self::Game => "stop(host=\"game\")",
            Self::Editor => "stop(host=\"editor\")",
        }
    }

    /// Human-readable launch tool name for error messages.
    #[must_use]
    pub const fn launch_tool_name(self) -> &'static str {
        match self {
            Self::Game => "launch(host=\"game\")",
            Self::Editor => "launch(host=\"editor\")",
        }
    }

    /// Built-in default port.
    #[must_use]
    pub const fn default_port(self) -> QaPort {
        match self {
            Self::Game => DEFAULT_GAME_PORT,
            Self::Editor => DEFAULT_EDITOR_PORT,
        }
    }

    /// Port from the channel env var, or the default.
    #[must_use]
    pub fn port_from_env(self) -> QaPort {
        std::env::var(self.default_spec().channel().port().as_str())
            .ok()
            .and_then(|value| value.trim().parse::<u16>().ok())
            .map_or_else(|| self.default_port(), QaPort::new)
    }

    /// Default launch recipe for this host.
    #[must_use]
    pub fn default_spec(self) -> LaunchSpec {
        match self {
            Self::Game => LaunchSpec::game_default(),
            Self::Editor => LaunchSpec::editor_default(),
        }
    }

    /// Boot/kill timing for this host.
    #[must_use]
    pub const fn lifecycle_config(self) -> LifecycleConfig {
        match self {
            Self::Game => LifecycleConfig::game(),
            Self::Editor => LifecycleConfig::editor(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::QaHost;
    use crate::lifecycle::LifecycleConfig;

    #[test]
    fn each_host_carries_its_own_lifecycle_config() {
        assert_eq!(QaHost::Game.lifecycle_config(), LifecycleConfig::game());
        assert_eq!(QaHost::Editor.lifecycle_config(), LifecycleConfig::editor());
        assert_eq!(
            QaHost::Game.lifecycle_config().boot_timeout(),
            QaHost::Editor.lifecycle_config().boot_timeout()
        );
    }

    #[test]
    fn the_two_hosts_have_distinct_default_ports() {
        assert_eq!(*QaHost::Game.default_port(), 7616);
        assert_eq!(*QaHost::Editor.default_port(), 7617);
        assert_ne!(QaHost::Game.default_port(), QaHost::Editor.default_port());
    }

    #[test]
    fn each_host_defaults_to_its_own_package_and_channel() {
        let game = QaHost::Game.default_spec();
        let editor = QaHost::Editor.default_spec();
        assert_eq!(game.package().as_str(), "game");
        assert_eq!(editor.package().as_str(), "editor");
        assert_eq!(game.channel().enable().as_str(), "GDTF_MCP");
        assert_eq!(editor.channel().enable().as_str(), "GDTF_EDITOR_MCP");
    }
}
