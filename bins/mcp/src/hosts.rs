//! The hosts this game's bridge drives: the game binary and the content editor.

use cobalt_mcp_server::{
    CargoPackage, EnvVarName, FeatureList, FeatureName, HostName, HostRegistry, LaunchPolicy,
    LifecycleConfig, McpChannel, McpHostSpec, McpPort,
};

/// The loopback port the game host's listener binds by default.
pub const DEFAULT_GAME_PORT: McpPort = McpPort::new(7616);
/// The loopback port the editor host's listener binds by default.
pub const DEFAULT_EDITOR_PORT: McpPort = McpPort::new(7617);

// The umbrella feature that turns on each host's QA channel.
const DEVELOPMENT_FEATURE: &str = "development";

// The feature list both of this game's hosts are built with.
fn development() -> FeatureList {
    FeatureList::new(vec![FeatureName::new(DEVELOPMENT_FEATURE.to_owned())])
}

// One host value from the parts that differ between this game's two hosts.
fn host(
    name: HostName,
    package: CargoPackage,
    port: McpPort,
    channel: McpChannel,
    policy: LaunchPolicy,
) -> McpHostSpec {
    McpHostSpec::new(
        name,
        package,
        development(),
        port,
        None,
        channel,
        LifecycleConfig::defaults_with_policy(policy),
    )
}

/// The game host, which keeps one child, and the editor host, which starts another each launch.
#[must_use]
pub fn registry() -> HostRegistry {
    HostRegistry::new(vec![
        host(
            HostName::new("game".to_owned()),
            CargoPackage::new("game".to_owned()),
            DEFAULT_GAME_PORT,
            McpChannel::new(
                EnvVarName::new("GDTF_MCP".to_owned()),
                EnvVarName::new("GDTF_MCP_PORT".to_owned()),
            ),
            LaunchPolicy::Reuse,
        ),
        host(
            HostName::new("editor".to_owned()),
            CargoPackage::new("editor".to_owned()),
            DEFAULT_EDITOR_PORT,
            McpChannel::new(
                EnvVarName::new("GDTF_EDITOR_MCP".to_owned()),
                EnvVarName::new("EDITOR_MCP_PORT".to_owned()),
            ),
            LaunchPolicy::AlwaysSpawn,
        ),
    ])
}
