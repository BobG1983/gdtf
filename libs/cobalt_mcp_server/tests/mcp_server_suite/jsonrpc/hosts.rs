//! The two hosts these tests register, and the identity the server advertises.

use cobalt_mcp_server::{
    CargoPackage, FeatureList, FeatureName, HostLifecycle, HostName, HostPair, HostRegistry,
    HostSet, LaunchPolicy, LifecycleConfig, McpHostSpec, McpLink, McpPort, ServerIdentity,
    ServerName, ServerVersion,
};

use crate::jsonrpc::support::{BRAMBLE_PORT, THISTLE_PORT};

// A host registered under `name`, on `port`, with the policy its instance rules follow.
fn registered(name: &str, port: u16, policy: LaunchPolicy) -> McpHostSpec {
    McpHostSpec::new(
        HostName::new(name.to_owned()),
        CargoPackage::new(format!("{name}_package")),
        FeatureList::new(vec![FeatureName::new(format!("{name}_feature"))]),
        McpPort::new(port),
        None,
        LifecycleConfig::defaults_with_policy(policy),
    )
}

/// The two hosts these tests register: one keeps its child, one starts another per launch.
pub(crate) fn test_registry() -> HostRegistry {
    HostRegistry::new(vec![
        registered("thistle", THISTLE_PORT, LaunchPolicy::Reuse),
        registered("bramble", BRAMBLE_PORT, LaunchPolicy::AlwaysSpawn),
    ])
}

/// What these tests advertise as the server's identity.
pub(crate) fn test_identity() -> ServerIdentity {
    ServerIdentity::new(
        ServerName::new("sample-bridge".to_owned()),
        ServerVersion::new("9.9.9".to_owned()),
    )
}

/// A host set holding the two registered hosts' links and lifecycles.
pub(crate) fn two_host_set<'a>(
    thistle_link: &'a mut dyn McpLink,
    thistle_life: &'a mut dyn HostLifecycle,
    bramble_link: &'a mut dyn McpLink,
    bramble_life: &'a mut dyn HostLifecycle,
) -> HostSet<'a> {
    HostSet::new(
        test_registry(),
        vec![
            (
                HostName::new("thistle".to_owned()),
                HostPair::new(thistle_link, thistle_life),
            ),
            (
                HostName::new("bramble".to_owned()),
                HostPair::new(bramble_link, bramble_life),
            ),
        ],
    )
}
