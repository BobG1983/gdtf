//! Borrowed link + lifecycle pair for each registered host.

use super::{name::HostName, registry::HostRegistry, spec::McpHostSpec};
use crate::{lifecycle::HostLifecycle, link::McpLink};

/// Link and lifecycle for one host.
pub struct HostPair<'a> {
    link:      &'a mut dyn McpLink,
    lifecycle: &'a mut dyn HostLifecycle,
}

impl<'a> HostPair<'a> {
    /// Pair a link with its lifecycle manager.
    pub fn new(link: &'a mut dyn McpLink, lifecycle: &'a mut dyn HostLifecycle) -> Self {
        Self { link, lifecycle }
    }

    /// Mutable link.
    pub fn link(&mut self) -> &mut dyn McpLink {
        self.link
    }

    /// Mutable lifecycle manager.
    pub fn lifecycle(&mut self) -> &mut dyn HostLifecycle {
        self.lifecycle
    }

    /// Both handles at once.
    pub fn parts(&mut self) -> (&mut dyn McpLink, &mut dyn HostLifecycle) {
        (self.link, self.lifecycle)
    }
}

/// Every registered host's pair, addressed by the name it was registered under.
pub struct HostSet<'a> {
    registry: HostRegistry,
    pairs:    Vec<(HostName, HostPair<'a>)>,
}

impl<'a> HostSet<'a> {
    /// Build from the registry and one pair per registered name.
    #[must_use]
    pub const fn new(registry: HostRegistry, pairs: Vec<(HostName, HostPair<'a>)>) -> Self {
        Self { registry, pairs }
    }

    /// The hosts this set was built from.
    #[must_use]
    pub const fn registry(&self) -> &HostRegistry {
        &self.registry
    }

    /// Pair registered under `name`.
    pub fn pair(&mut self, name: &HostName) -> Option<&mut HostPair<'a>> {
        self.pairs
            .iter_mut()
            .find(|(held, _)| held == name)
            .map(|(_, pair)| pair)
    }

    /// The host a call named, or the default when it named none, with its pair.
    ///
    /// # Errors
    ///
    /// Returns a message when the name is not registered, or when it is registered but no pair
    /// was supplied for it.
    pub fn resolve(
        &mut self,
        requested: Option<&HostName>,
    ) -> Result<(McpHostSpec, &mut HostPair<'a>), String> {
        let spec = self.registry.resolve(requested)?.clone();
        let pair = self
            .pairs
            .iter_mut()
            .find(|(held, _)| held == spec.name())
            .map(|(_, pair)| pair)
            .ok_or_else(|| {
                format!(
                    "host {:?} is registered but this server holds no link for it",
                    spec.name().as_str()
                )
            })?;
        Ok((spec, pair))
    }
}

#[cfg(test)]
mod test {
    use cobalt_mcp_protocol::{
        message::{HelloFacts, McpRequest, McpResponse, ProtocolVersion, ServerNameNet},
        ports::McpPort,
    };

    const STUB_PORT: McpPort = McpPort::new(4100);

    const STUB_PROTOCOL: ProtocolVersion = ProtocolVersion::new(1);

    use super::{HostName, HostPair, HostSet};
    use crate::{
        error::McpError,
        hosts::test_support::{named, two_hosts},
        lifecycle::{
            ChildPid, HostLifecycle, InstanceId, LaunchOutcome, LaunchPolicy, LaunchSpec,
            OutputTail, RecordedInstance, StopOutcome, TailLines, WorkingDir,
        },
        link::McpLink,
    };

    struct NamedLink(&'static str);

    impl McpLink for NamedLink {
        fn request(&mut self, _request: McpRequest) -> Result<McpResponse, McpError> {
            Ok(McpResponse::HelloOk(HelloFacts::new(
                STUB_PROTOCOL,
                ServerNameNet::new(self.0.to_owned()),
            )))
        }
    }

    struct NamedLifecycle(u32);

    impl HostLifecycle for NamedLifecycle {
        fn launch(&mut self, port: McpPort, _spec: &LaunchSpec) -> LaunchOutcome {
            LaunchOutcome::Launched {
                port,
                pid: ChildPid::new(self.0),
                instance: InstanceId::new(format!("instance-{}", self.0)),
            }
        }

        fn stop(&mut self, _port: McpPort) -> StopOutcome {
            self.stop_owned()
        }

        fn stop_instance(&mut self, _instance: &InstanceId) -> StopOutcome {
            StopOutcome::NotRunning
        }

        fn stop_owned(&mut self) -> StopOutcome {
            StopOutcome::Stopped {
                pid: ChildPid::new(self.0),
            }
        }

        fn reap_dead_child(&mut self) {}

        fn instances(&self) -> Vec<RecordedInstance> {
            Vec::new()
        }

        fn child_working_dir(&self) -> Option<WorkingDir> {
            None
        }

        fn instance_working_dir(&self, _instance: &InstanceId) -> Option<WorkingDir> {
            None
        }

        fn child_output(&self, _max: TailLines) -> Option<OutputTail> {
            None
        }

        fn instance_output(&self, _instance: &InstanceId, _max: TailLines) -> Option<OutputTail> {
            None
        }
    }

    fn answered_by(pair: &mut HostPair<'_>) -> ServerNameNet {
        let Ok(McpResponse::HelloOk(facts)) = pair.link().request(McpRequest::Hello(STUB_PROTOCOL))
        else {
            unreachable!("the named link always answers HelloOk");
        };
        facts.server
    }

    fn stopped_pid(pair: &mut HostPair<'_>) -> u32 {
        let StopOutcome::Stopped { pid } = pair.lifecycle().stop(STUB_PORT) else {
            unreachable!("the named lifecycle always reports a stopped pid");
        };
        *pid
    }

    fn set_of_two<'a>(
        alpha_link: &'a mut NamedLink,
        alpha_life: &'a mut NamedLifecycle,
        beta_link: &'a mut NamedLink,
        beta_life: &'a mut NamedLifecycle,
    ) -> HostSet<'a> {
        HostSet::new(
            two_hosts(),
            vec![
                (
                    HostName::new("alpha".to_owned()),
                    HostPair::new(alpha_link, alpha_life),
                ),
                (
                    HostName::new("beta".to_owned()),
                    HostPair::new(beta_link, beta_life),
                ),
            ],
        )
    }

    #[test]
    fn each_registered_name_resolves_to_its_own_link_lifecycle_and_policy() {
        let (mut alpha_link, mut beta_link) = (NamedLink("alpha"), NamedLink("beta"));
        let (mut alpha_life, mut beta_life) = (NamedLifecycle(4100), NamedLifecycle(4200));
        let mut hosts = set_of_two(
            &mut alpha_link,
            &mut alpha_life,
            &mut beta_link,
            &mut beta_life,
        );

        let Ok((alpha, alpha_pair)) = hosts.resolve(Some(&named("alpha"))) else {
            unreachable!("alpha is registered");
        };
        assert_eq!(alpha.launch_policy(), LaunchPolicy::Reuse);
        assert_eq!(
            answered_by(alpha_pair),
            ServerNameNet::new("alpha".to_owned())
        );
        assert_eq!(stopped_pid(alpha_pair), 4100);

        let Ok((beta, beta_pair)) = hosts.resolve(Some(&named("beta"))) else {
            unreachable!("beta is registered");
        };
        assert_eq!(beta.launch_policy(), LaunchPolicy::AlwaysSpawn);
        assert_eq!(
            answered_by(beta_pair),
            ServerNameNet::new("beta".to_owned())
        );
        assert_eq!(stopped_pid(beta_pair), 4200);
    }

    #[test]
    fn a_call_naming_no_host_lands_on_the_first_registered_pair() {
        let (mut alpha_link, mut beta_link) = (NamedLink("alpha"), NamedLink("beta"));
        let (mut alpha_life, mut beta_life) = (NamedLifecycle(4100), NamedLifecycle(4200));
        let mut hosts = set_of_two(
            &mut alpha_link,
            &mut alpha_life,
            &mut beta_link,
            &mut beta_life,
        );

        let Ok((implied, pair)) = hosts.resolve(None) else {
            unreachable!("the default host resolves");
        };
        assert_eq!(implied.name().as_str(), "alpha");
        assert_eq!(stopped_pid(pair), 4100);
    }

    #[test]
    fn an_unregistered_name_is_refused_rather_than_served_by_the_default() {
        let (mut alpha_link, mut beta_link) = (NamedLink("alpha"), NamedLink("beta"));
        let (mut alpha_life, mut beta_life) = (NamedLifecycle(4100), NamedLifecycle(4200));
        let mut hosts = set_of_two(
            &mut alpha_link,
            &mut alpha_life,
            &mut beta_link,
            &mut beta_life,
        );

        assert!(hosts.resolve(Some(&named("gamma"))).is_err());
    }
}
