//! The tunable timing knobs the game-lifecycle manager waits on (GTW-745).
//!
//! [`LifecycleConfig`] gathers the four `Duration` knobs — how long a launch waits for
//! readiness, how often it polls, how long a graceful stop waits before SIGKILL, and the
//! per-probe socket deadline. Production uses [`LifecycleConfig::default`]; tests build a
//! short-fused config so the timeout / stop paths run in well under a second.

use core::time::Duration;

use super::values::{BootTimeout, KillGrace, PollInterval, ProbeTimeout};

/// The production boot timeout — generous, because the very first `cargo run` compiles
/// the game before it can bind its listener.
const DEFAULT_BOOT_TIMEOUT: BootTimeout = BootTimeout::new(Duration::from_secs(180));

/// The production readiness poll cadence.
const DEFAULT_POLL_INTERVAL: PollInterval = PollInterval::new(Duration::from_millis(250));

/// The production SIGTERM→SIGKILL grace period.
const DEFAULT_KILL_GRACE: KillGrace = KillGrace::new(Duration::from_secs(5));

/// The production per-probe socket deadline.
const DEFAULT_PROBE_TIMEOUT: ProbeTimeout = ProbeTimeout::new(Duration::from_millis(500));

/// The timing knobs the launch / stop logic reads.
///
/// A plain aggregate of the four typed durations (not a newtype — it has more than one
/// field), read through its accessors so the manager never touches a bare `Duration`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleConfig {
    /// How long a launch waits for the readiness handshake.
    boot_timeout:  BootTimeout,
    /// How often the launch loop probes readiness.
    poll_interval: PollInterval,
    /// How long a graceful stop waits after SIGTERM before SIGKILL.
    kill_grace:    KillGrace,
    /// The connect / read deadline on one readiness probe.
    probe_timeout: ProbeTimeout,
}

impl LifecycleConfig {
    /// Build a config from its four timing knobs.
    #[must_use]
    pub const fn new(
        boot_timeout: BootTimeout,
        poll_interval: PollInterval,
        kill_grace: KillGrace,
        probe_timeout: ProbeTimeout,
    ) -> Self {
        Self {
            boot_timeout,
            poll_interval,
            kill_grace,
            probe_timeout,
        }
    }

    /// How long a launch waits for the child to answer readiness.
    #[must_use]
    pub const fn boot_timeout(&self) -> BootTimeout {
        self.boot_timeout
    }

    /// How often the launch loop probes readiness.
    #[must_use]
    pub const fn poll_interval(&self) -> PollInterval {
        self.poll_interval
    }

    /// How long a graceful stop waits after SIGTERM before SIGKILL.
    #[must_use]
    pub const fn kill_grace(&self) -> KillGrace {
        self.kill_grace
    }

    /// The connect / read deadline on one readiness probe.
    #[must_use]
    pub const fn probe_timeout(&self) -> ProbeTimeout {
        self.probe_timeout
    }
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self::new(
            DEFAULT_BOOT_TIMEOUT,
            DEFAULT_POLL_INTERVAL,
            DEFAULT_KILL_GRACE,
            DEFAULT_PROBE_TIMEOUT,
        )
    }
}
