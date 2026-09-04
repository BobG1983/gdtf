//! Boot and kill timing for a host.

use core::time::Duration;

use super::values::{BootTimeout, KillGrace, PollInterval, ProbeTimeout, SweepInterval};

const DEFAULT_BOOT_TIMEOUT: BootTimeout = BootTimeout::new(Duration::from_secs(180));

const DEFAULT_POLL_INTERVAL: PollInterval = PollInterval::new(Duration::from_millis(250));

const DEFAULT_KILL_GRACE: KillGrace = KillGrace::new(Duration::from_secs(5));

const DEFAULT_PROBE_TIMEOUT: ProbeTimeout = ProbeTimeout::new(Duration::from_millis(500));

const DEFAULT_SWEEP_INTERVAL: SweepInterval = SweepInterval::new(Duration::from_secs(60));

/// What a launch does about the children a host already records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchPolicy {
    /// Keep the recorded child and report it as already running.
    Reuse,
    /// Start another child on every launch.
    AlwaysSpawn,
}

/// Timing knobs for launch readiness and stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleConfig {
    boot_timeout:   BootTimeout,
    poll_interval:  PollInterval,
    kill_grace:     KillGrace,
    probe_timeout:  ProbeTimeout,
    sweep_interval: SweepInterval,
    launch_policy:  LaunchPolicy,
}

impl LifecycleConfig {
    /// Build from explicit timings and a launch policy.
    #[must_use]
    pub const fn new(
        boot_timeout: BootTimeout,
        poll_interval: PollInterval,
        kill_grace: KillGrace,
        probe_timeout: ProbeTimeout,
        sweep_interval: SweepInterval,
        launch_policy: LaunchPolicy,
    ) -> Self {
        Self {
            boot_timeout,
            poll_interval,
            kill_grace,
            probe_timeout,
            sweep_interval,
            launch_policy,
        }
    }

    /// How long to wait for readiness after spawn.
    #[must_use]
    pub const fn boot_timeout(&self) -> BootTimeout {
        self.boot_timeout
    }

    /// Poll interval during boot/kill waits.
    #[must_use]
    pub const fn poll_interval(&self) -> PollInterval {
        self.poll_interval
    }

    /// Grace after terminate before kill.
    #[must_use]
    pub const fn kill_grace(&self) -> KillGrace {
        self.kill_grace
    }

    /// Timeout for one readiness probe.
    #[must_use]
    pub const fn probe_timeout(&self) -> ProbeTimeout {
        self.probe_timeout
    }

    /// How often to check the recorded child is still alive.
    #[must_use]
    pub const fn sweep_interval(&self) -> SweepInterval {
        self.sweep_interval
    }

    /// Whether a launch reuses a recorded child or starts another.
    #[must_use]
    pub const fn launch_policy(&self) -> LaunchPolicy {
        self.launch_policy
    }

    /// The shipped timings under an explicit launch policy.
    #[must_use]
    pub const fn defaults_with_policy(launch_policy: LaunchPolicy) -> Self {
        Self::new(
            DEFAULT_BOOT_TIMEOUT,
            DEFAULT_POLL_INTERVAL,
            DEFAULT_KILL_GRACE,
            DEFAULT_PROBE_TIMEOUT,
            DEFAULT_SWEEP_INTERVAL,
            launch_policy,
        )
    }
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self::defaults_with_policy(LaunchPolicy::Reuse)
    }
}

#[cfg(test)]
mod test {
    use super::{LaunchPolicy, LifecycleConfig};
    use crate::hosts::test_support::{named, two_hosts};

    #[test]
    fn the_policy_is_the_only_thing_the_shipped_timings_differ_by() {
        let reuse = LifecycleConfig::defaults_with_policy(LaunchPolicy::Reuse);
        let always = LifecycleConfig::defaults_with_policy(LaunchPolicy::AlwaysSpawn);

        assert_eq!(reuse.boot_timeout(), always.boot_timeout());
        assert_eq!(reuse.sweep_interval(), always.sweep_interval());
        assert_eq!(reuse.launch_policy(), LaunchPolicy::Reuse);
        assert_eq!(always.launch_policy(), LaunchPolicy::AlwaysSpawn);
        assert_eq!(LifecycleConfig::default(), reuse);
    }

    #[test]
    fn a_registered_host_carries_the_policy_it_was_registered_with() {
        let registry = two_hosts();

        let (Some(reusing), Some(spawning)) =
            (registry.get(&named("alpha")), registry.get(&named("beta")))
        else {
            unreachable!("both hosts are registered");
        };
        assert_eq!(
            reusing.lifecycle_config().launch_policy(),
            LaunchPolicy::Reuse,
            "a host registered to keep its child reports a second launch as already running"
        );
        assert_eq!(
            spawning.lifecycle_config().launch_policy(),
            LaunchPolicy::AlwaysSpawn,
            "a host registered to start another child does so on every launch"
        );
    }
}
