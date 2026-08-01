//! [`QaHost`] — which of the two child processes a tool call is about (GTW-808).

use crate::{
    lifecycle::{LaunchSpec, LifecycleConfig},
    link::QaPort,
};

/// The game's default loopback port, mirroring `crates/gdtf_app/src/dev/net_qa/config.rs`.
const DEFAULT_GAME_PORT: QaPort = QaPort::new(7616);

/// The editor's default loopback port, mirroring
/// `crates/gdtf_content_editor/src/net_qa/config.rs` — deliberately one above the game's,
/// so both hosts can be up at once without fighting for a socket.
const DEFAULT_EDITOR_PORT: QaPort = QaPort::new(7617);

/// One of the two child processes this MCP host launches, drives, and stops.
///
/// A single `gdtf_qa_mcp` binary manages both (the user's 2026-07-24 ruling on GTW-786),
/// so nearly everything below the tool layer is written once and told WHICH host it is
/// acting for. This enum is that answer: it carries the per-host port default, the default
/// launch recipe, and the timing config, so no other module branches on "game or editor".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QaHost {
    /// The game binary (`grimdark_turfwar`) on its `GDTF_NET_QA_PORT`.
    Game,
    /// The content editor binary (`gdtf_content_editor_bin`) on its
    /// `GDTF_EDITOR_NET_QA_PORT` (GTW-808).
    Editor,
}

impl QaHost {
    /// Every host, in the order the tool registry lists their tools.
    pub const ALL: [Self; 2] = [Self::Game, Self::Editor];

    /// The word this host is called in a message a caller reads.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::Editor => "editor",
        }
    }

    /// Resolve a caller-supplied host word back to the host, or `None` for a word that names
    /// neither.
    ///
    /// The inverse of [`label`](Self::label), and the ONE place a `host` tool argument becomes
    /// a host: EVERY tool reads its `host` argument through here, so the accepted spellings
    /// and the ones a schema advertises can never drift apart.
    #[must_use]
    pub fn from_label(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|host| host.label() == word)
    }

    /// The MCP call that stops this host's child — named in the recipe-mismatch message, so
    /// the caller is told exactly what to send rather than which tool to look up.
    #[must_use]
    pub const fn stop_tool_name(self) -> &'static str {
        match self {
            Self::Game => "stop(host=\"game\")",
            Self::Editor => "stop(host=\"editor\")",
        }
    }

    /// The port this host binds when nothing names one.
    #[must_use]
    pub const fn default_port(self) -> QaPort {
        match self {
            Self::Game => DEFAULT_GAME_PORT,
            Self::Editor => DEFAULT_EDITOR_PORT,
        }
    }

    /// The port from this host's own port variable, or [`default_port`](Self::default_port)
    /// when it is unset or does not parse as a `u16` — the same rule the child applies to
    /// the same variable, so both ends agree without configuration.
    #[must_use]
    pub fn port_from_env(self) -> QaPort {
        std::env::var(self.default_spec().channel().port().as_str())
            .ok()
            .and_then(|value| value.trim().parse::<u16>().ok())
            .map_or_else(|| self.default_port(), QaPort::new)
    }

    /// The recipe a launch call runs when it names nothing of its own.
    #[must_use]
    pub fn default_spec(self) -> LaunchSpec {
        match self {
            Self::Game => LaunchSpec::game_default(),
            Self::Editor => LaunchSpec::editor_default(),
        }
    }

    /// The timing knobs this host's manager waits on — they differ in the boot timeout,
    /// because an editor launch is a cold build far more often (GTW-808 clause 7).
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

    /// Each host carries its OWN timing config — the editor's longer boot timeout only
    /// helps if the editor host is the one that asks for it (GTW-808 clause 7).
    #[test]
    fn each_host_carries_its_own_lifecycle_config() {
        assert_eq!(QaHost::Game.lifecycle_config(), LifecycleConfig::game());
        assert_eq!(QaHost::Editor.lifecycle_config(), LifecycleConfig::editor());
        assert_ne!(
            QaHost::Game.lifecycle_config().boot_timeout(),
            QaHost::Editor.lifecycle_config().boot_timeout()
        );
    }

    /// The two hosts never share a default port — the pair is meant to run at once
    /// (GTW-808 clause 5).
    #[test]
    fn the_two_hosts_have_distinct_default_ports() {
        assert_eq!(*QaHost::Game.default_port(), 7616);
        assert_eq!(*QaHost::Editor.default_port(), 7617);
        assert_ne!(QaHost::Game.default_port(), QaHost::Editor.default_port());
    }

    /// Each host's default recipe builds its own package over its own QA channel.
    #[test]
    fn each_host_defaults_to_its_own_package_and_channel() {
        let game = QaHost::Game.default_spec();
        let editor = QaHost::Editor.default_spec();
        assert_eq!(game.package().as_str(), "grimdark_turfwar");
        assert_eq!(editor.package().as_str(), "gdtf_content_editor_bin");
        assert_eq!(game.channel().enable().as_str(), "GDTF_NET_QA");
        assert_eq!(editor.channel().enable().as_str(), "GDTF_EDITOR_NET_QA");
    }
}
