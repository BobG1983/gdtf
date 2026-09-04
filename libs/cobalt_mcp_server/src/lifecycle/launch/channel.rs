//! Env var names that enable and configure a host's MCP channel.

use super::values::EnvVarName;

/// Enable flag and port env var names for a host.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct McpChannel {
    enable: EnvVarName,
    port:   EnvVarName,
}

impl McpChannel {
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
}

#[cfg(test)]
mod test {
    use crate::hosts::test_support::{named, two_hosts};

    #[test]
    fn two_registered_hosts_share_no_channel_variable_name() {
        let registry = two_hosts();

        let (Some(first), Some(second)) =
            (registry.get(&named("alpha")), registry.get(&named("beta")))
        else {
            unreachable!("both hosts are registered");
        };
        assert_ne!(first.channel().enable(), second.channel().enable());
        assert_ne!(first.channel().port(), second.channel().port());
        assert_eq!(first.channel().enable().as_str(), "ALPHA_CHANNEL");
        assert_eq!(first.channel().port().as_str(), "ALPHA_CHANNEL_PORT");
    }
}
