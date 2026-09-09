//! Registered host values the unit tests in this module tree share.

use cobalt_mcp_protocol::ports::McpPort;

use super::{name::HostName, registry::HostRegistry, spec::McpHostSpec};
use crate::lifecycle::{CargoPackage, FeatureList, FeatureName, LaunchPolicy, LifecycleConfig};

// A registered name as a tool call spells it.
pub(crate) fn named(name: &str) -> HostName {
    HostName::new(name.to_owned())
}

// A host whose every field is derived from its name, so a wrong lookup shows in any field.
pub(crate) fn registered(name: &str, port: u16, many_instances: bool) -> McpHostSpec {
    let policy = if many_instances {
        LaunchPolicy::AlwaysSpawn
    } else {
        LaunchPolicy::Reuse
    };
    McpHostSpec::new(
        HostName::new(name.to_owned()),
        CargoPackage::new(format!("{name}_package")),
        FeatureList::new(vec![FeatureName::new(format!("{name}_feature"))]),
        McpPort::new(port),
        None,
        LifecycleConfig::defaults_with_policy(policy),
    )
}

// Two hosts under names no shipped string uses: one keeps a child, one starts another each time.
pub(crate) fn two_hosts() -> HostRegistry {
    HostRegistry::new(vec![
        registered("alpha", 4100, false),
        registered("beta", 4200, true),
    ])
}
