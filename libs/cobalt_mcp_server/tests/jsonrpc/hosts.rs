//! The two hosts these tests register, and the identity the server advertises.

use cobalt_mcp_server::{
    CargoPackage, EnvVarName, FeatureList, FeatureName, HostLifecycle, HostName, HostPair,
    HostRegistry, HostSet, LaunchPolicy, LifecycleConfig, QaChannel, QaHostSpec, QaLink, QaPort,
    ServerIdentity, ServerName, ServerVersion,
};

use crate::support::{BRAMBLE_PORT, THISTLE_PORT};

// A host registered under `name`, on `port`, with the policy its instance rules follow.
fn registered(name: &str, port: u16, policy: LaunchPolicy) -> QaHostSpec {
    let upper = name.to_uppercase();
    QaHostSpec::new(
        HostName::new(name.to_owned()),
        CargoPackage::new(format!("{name}_package")),
        FeatureList::new(vec![FeatureName::new(format!("{name}_feature"))]),
        QaPort::new(port),
        None,
        QaChannel::new(
            EnvVarName::new(format!("{upper}_CHANNEL")),
            EnvVarName::new(format!("{upper}_CHANNEL_PORT")),
        ),
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
    thistle_link: &'a mut dyn QaLink,
    thistle_life: &'a mut dyn HostLifecycle,
    bramble_link: &'a mut dyn QaLink,
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
