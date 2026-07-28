//! The tunable timing knobs the game-lifecycle manager waits on (GTW-745).
//!
//! [`LifecycleConfig`] gathers the four `Duration` knobs — how long a launch waits for
//! readiness, how often it polls, how long a graceful stop waits before SIGKILL, and the
//! per-probe socket deadline. Production uses [`LifecycleConfig::game`] or
//! [`LifecycleConfig::editor`]; tests build a short-fused config so the timeout / stop
//! paths run in well under a second.
//!
//! The two production configs differ in ONE knob, the boot timeout, and only because a
//! first launch of a recipe waits out a `cargo` build before the child can bind anything.
//! The editor's `dynamic_linking,net_qa` combination is one nothing else in the repo
//! builds — `cargo dbuild` builds the GAME binary, and the workspace checks never link an
//! editor binary — so an editor launch is a cold build far more often than a game launch
//! is, and 180 seconds is not enough for one (GTW-808 clause 7).

use core::time::Duration;

use super::values::{BootTimeout, KillGrace, PollInterval, ProbeTimeout};

/// The GAME's production boot timeout — generous, because the very first `cargo run`
/// compiles the game before it can bind its listener.
const GAME_BOOT_TIMEOUT: BootTimeout = BootTimeout::new(Duration::from_secs(180));

/// The EDITOR's production boot timeout (GTW-808 clause 7).
///
/// Ten minutes, not the game's three: an editor launch builds a feature combination the
/// dev loop does not otherwise produce, so a cold `dynamic_linking,net_qa` editor build
/// from scratch is the common case rather than the exception, and a build that is still
/// compiling is not a hung editor. The timeout failure message names the build as the
/// likely cause and gives the warm-up command, so a launch that does hit this limit is
/// diagnosable rather than a bare "timed out".
const EDITOR_BOOT_TIMEOUT: BootTimeout = BootTimeout::new(Duration::from_secs(600));

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

    /// The production config for a GAME child.
    #[must_use]
    pub const fn game() -> Self {
        Self::new(
            GAME_BOOT_TIMEOUT,
            DEFAULT_POLL_INTERVAL,
            DEFAULT_KILL_GRACE,
            DEFAULT_PROBE_TIMEOUT,
        )
    }

    /// The production config for an EDITOR child — the game's, with a longer boot
    /// timeout (see the module doc).
    #[must_use]
    pub const fn editor() -> Self {
        Self::new(
            EDITOR_BOOT_TIMEOUT,
            DEFAULT_POLL_INTERVAL,
            DEFAULT_KILL_GRACE,
            DEFAULT_PROBE_TIMEOUT,
        )
    }
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self::game()
    }
}

#[cfg(test)]
mod test {
    use super::LifecycleConfig;

    /// The editor waits longer for readiness than the game — the cold-build allowance
    /// GTW-808 clause 7 requires.
    #[test]
    fn the_editor_waits_longer_for_readiness_than_the_game() {
        assert!(
            *LifecycleConfig::editor().boot_timeout() > *LifecycleConfig::game().boot_timeout()
        );
        assert_eq!(LifecycleConfig::default(), LifecycleConfig::game());
    }
}
