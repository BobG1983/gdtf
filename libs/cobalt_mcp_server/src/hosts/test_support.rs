//! Registered host values the unit tests in this module tree share.

use super::{name::HostName, registry::HostRegistry, spec::QaHostSpec};
use crate::{
    lifecycle::{
        CargoPackage, EnvVarName, FeatureList, FeatureName, LaunchPolicy, LifecycleConfig,
        QaChannel,
    },
    link::QaPort,
};

// A registered name as a tool call spells it.
pub(crate) fn named(name: &str) -> HostName {
    HostName::new(name.to_owned())
}

// A host whose every field is derived from its name, so a wrong lookup shows in any field.
pub(crate) fn registered(name: &str, port: u16, many_instances: bool) -> QaHostSpec {
    let upper = name.to_uppercase();
    let policy = if many_instances {
        LaunchPolicy::AlwaysSpawn
    } else {
        LaunchPolicy::Reuse
    };
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

// Two hosts under names no shipped string uses: one keeps a child, one starts another each time.
pub(crate) fn two_hosts() -> HostRegistry {
    HostRegistry::new(vec![
        registered("alpha", 4100, false),
        registered("beta", 4200, true),
    ])
}
