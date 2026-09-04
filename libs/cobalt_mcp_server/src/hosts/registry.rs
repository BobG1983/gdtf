//! The hosts this server was started with, in registration order.

use super::{name::HostName, spec::QaHostSpec};

/// Every host the caller registered, first one first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostRegistry {
    hosts: Vec<QaHostSpec>,
}

impl HostRegistry {
    /// Register these hosts, in this order.
    #[must_use]
    pub const fn new(hosts: Vec<QaHostSpec>) -> Self {
        Self { hosts }
    }

    /// Every registered host.
    #[must_use]
    pub fn hosts(&self) -> &[QaHostSpec] {
        &self.hosts
    }

    /// Host a call that names none acts on: the first registered.
    #[must_use]
    pub fn default_host(&self) -> Option<&QaHostSpec> {
        self.hosts.first()
    }

    /// Host registered under `name`.
    #[must_use]
    pub fn get(&self, name: &HostName) -> Option<&QaHostSpec> {
        self.hosts.iter().find(|host| host.name() == name)
    }

    /// Every registered name, in registration order.
    #[must_use]
    pub fn names(&self) -> Vec<&HostName> {
        self.hosts.iter().map(QaHostSpec::name).collect()
    }

    /// How a description or error message lists the registered names.
    #[must_use]
    pub fn quoted_names(&self) -> Vec<String> {
        self.hosts
            .iter()
            .map(|host| format!("{:?}", host.name().as_str()))
            .collect()
    }

    /// Hosts that keep several children at once.
    #[must_use]
    pub fn multi_instance_hosts(&self) -> Vec<&QaHostSpec> {
        self.hosts
            .iter()
            .filter(|host| host.runs_many_instances())
            .collect()
    }

    /// Hosts that keep one child and report a second launch as already running.
    #[must_use]
    pub fn single_instance_hosts(&self) -> Vec<&QaHostSpec> {
        self.hosts
            .iter()
            .filter(|host| !host.runs_many_instances())
            .collect()
    }

    /// The host a call named, or the default when it named none.
    ///
    /// # Errors
    ///
    /// Returns a message naming the registered hosts when the call named one that is not
    /// registered, or when nothing is registered at all.
    pub fn resolve(&self, requested: Option<&HostName>) -> Result<&QaHostSpec, String> {
        match requested {
            None => self
                .default_host()
                .ok_or_else(|| "this server has no host registered".to_owned()),
            Some(name) => self.get(name).ok_or_else(|| {
                format!(
                    "`host` must be one of: {}, not {:?}",
                    self.quoted_names().join(", "),
                    name.as_str()
                )
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::HostRegistry;
    use crate::hosts::test_support::{named, registered, two_hosts};

    #[test]
    fn a_call_that_names_no_host_lands_on_the_first_registered_one() {
        let registry = two_hosts();

        let Ok(resolved) = registry.resolve(None) else {
            unreachable!("a registry with hosts resolves the default");
        };
        assert_eq!(resolved.name().as_str(), "alpha");
        assert_eq!(
            registry.names().len(),
            2,
            "both registered hosts stay addressable"
        );
    }

    #[test]
    fn one_registered_host_is_the_only_choice_and_the_implied_one() {
        let registry = HostRegistry::new(vec![registered("solo", 4100, false)]);

        let Ok(implied) = registry.resolve(None) else {
            unreachable!("the one registered host is the default");
        };
        assert_eq!(implied.name().as_str(), "solo");
        assert_eq!(registry.quoted_names(), vec!["\"solo\"".to_owned()]);
    }

    #[test]
    fn a_name_that_was_never_registered_is_refused_rather_than_defaulted() {
        let registry = two_hosts();

        let Err(message) = registry.resolve(Some(&named("beta-two"))) else {
            unreachable!("an unregistered name is refused");
        };
        assert!(message.contains("\"alpha\""), "message: {message}");
        assert!(message.contains("beta-two"), "message: {message}");
    }

    #[test]
    fn each_registered_host_keeps_its_own_package_channel_port_and_policy() {
        let registry = two_hosts();

        let (Some(alpha), Some(beta)) =
            (registry.get(&named("alpha")), registry.get(&named("beta")))
        else {
            unreachable!("both registered hosts resolve by name");
        };
        assert_ne!(alpha.package(), beta.package());
        assert_ne!(alpha.channel().enable(), beta.channel().enable());
        assert_ne!(alpha.default_port(), beta.default_port());
        assert!(!alpha.runs_many_instances());
        assert!(beta.runs_many_instances());
        assert_eq!(registry.multi_instance_hosts().len(), 1);
        assert_eq!(registry.single_instance_hosts().len(), 1);
    }
}
