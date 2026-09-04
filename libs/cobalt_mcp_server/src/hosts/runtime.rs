//! The link and lifecycle this server owns for each registered host.

use super::{name::HostName, registry::HostRegistry, set::HostPair, spec::McpHostSpec};
use crate::{
    lifecycle::{CargoSpawner, HostLifecycle, HostManager, StopOutcome},
    link::McpClient,
};

/// One registered host's own link and lifecycle manager.
pub struct HostRuntime {
    name:      HostName,
    link:      McpClient,
    lifecycle: HostManager,
}

impl HostRuntime {
    /// Build the link and lifecycle this host's own value calls for.
    #[must_use]
    pub fn for_host(host: &McpHostSpec) -> Self {
        Self {
            name:      host.name().clone(),
            link:      McpClient::for_host(host),
            lifecycle: HostManager::with_config(
                Box::new(CargoSpawner::new()),
                host.lifecycle_config(),
            ),
        }
    }

    /// Name this runtime answers to.
    #[must_use]
    pub const fn name(&self) -> &HostName {
        &self.name
    }

    /// The link this runtime reaches its host over.
    #[must_use]
    pub const fn link(&self) -> &McpClient {
        &self.link
    }

    /// The lifecycle manager that starts and stops this host's children.
    #[must_use]
    pub const fn lifecycle(&self) -> &HostManager {
        &self.lifecycle
    }

    /// Stop every child this runtime's lifecycle started.
    pub fn stop_children(&mut self) -> StopOutcome {
        self.lifecycle.stop_owned()
    }

    /// Borrow this runtime as the pair its registered name addresses.
    pub fn pair(&mut self) -> (HostName, HostPair<'_>) {
        (
            self.name.clone(),
            HostPair::new(&mut self.link, &mut self.lifecycle),
        )
    }
}

/// One runtime per registered host, in registration order.
#[must_use]
pub fn runtimes(registry: &HostRegistry) -> Vec<HostRuntime> {
    registry.hosts().iter().map(HostRuntime::for_host).collect()
}

/// Every runtime's pair, each under the name its own host was registered with.
pub fn pairs(runtimes: &mut [HostRuntime]) -> Vec<(HostName, HostPair<'_>)> {
    runtimes.iter_mut().map(HostRuntime::pair).collect()
}

#[cfg(test)]
mod test {
    use super::{pairs, runtimes};
    use crate::{
        hosts::test_support::{named, two_hosts},
        lifecycle::LaunchPolicy,
    };

    #[test]
    fn each_registered_host_gets_a_link_and_a_lifecycle_built_from_its_own_value() {
        let registry = two_hosts();

        let built = runtimes(&registry);

        assert_eq!(built.len(), registry.hosts().len());
        for (runtime, host) in built.iter().zip(registry.hosts()) {
            assert_eq!(runtime.name(), host.name());
            assert_eq!(
                runtime.link().channel(),
                host.channel().enable(),
                "the link names the env var that enables this host's own channel"
            );
            assert_eq!(
                runtime.link().port(),
                host.port_from_env(),
                "the link is aimed at this host's own port"
            );
            assert_eq!(
                runtime.lifecycle().launch_policy(),
                host.launch_policy(),
                "the lifecycle keeps or replaces children the way this host was registered to"
            );
        }
    }

    #[test]
    fn a_host_that_starts_another_child_per_launch_never_shares_a_pair_with_one_that_reuses() {
        let registry = two_hosts();
        let mut built = runtimes(&registry);

        let addressed: Vec<_> = pairs(&mut built)
            .into_iter()
            .map(|(name, _)| name)
            .collect();

        assert_eq!(addressed, vec![named("alpha"), named("beta")]);
        let [reusing, spawning] = [0, 1].map(|index| &built[index]);
        assert_eq!(reusing.lifecycle().launch_policy(), LaunchPolicy::Reuse);
        assert_eq!(
            spawning.lifecycle().launch_policy(),
            LaunchPolicy::AlwaysSpawn
        );
        assert_ne!(reusing.link().port(), spawning.link().port());
    }
}
