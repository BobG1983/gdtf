//! Boot and kill timing for a host.

use core::time::Duration;

use super::values::{BootTimeout, KillGrace, PollInterval, ProbeTimeout};

const GAME_BOOT_TIMEOUT: BootTimeout = BootTimeout::new(Duration::from_secs(180));

const EDITOR_BOOT_TIMEOUT: BootTimeout = BootTimeout::new(Duration::from_secs(180));

const DEFAULT_POLL_INTERVAL: PollInterval = PollInterval::new(Duration::from_millis(250));

const DEFAULT_KILL_GRACE: KillGrace = KillGrace::new(Duration::from_secs(5));

const DEFAULT_PROBE_TIMEOUT: ProbeTimeout = ProbeTimeout::new(Duration::from_millis(500));

/// Timing knobs for launch readiness and stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleConfig {
    boot_timeout:  BootTimeout,
    poll_interval: PollInterval,
    kill_grace:    KillGrace,
    probe_timeout: ProbeTimeout,
}

impl LifecycleConfig {
    /// Build from explicit timings.
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

    /// Defaults for the game host.
    #[must_use]
    pub const fn game() -> Self {
        Self::new(
            GAME_BOOT_TIMEOUT,
            DEFAULT_POLL_INTERVAL,
            DEFAULT_KILL_GRACE,
            DEFAULT_PROBE_TIMEOUT,
        )
    }

    /// Defaults for the editor host (longer boot wait).
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

    #[test]
    fn editor_and_game_share_the_same_boot_timeout() {
        assert_eq!(
            LifecycleConfig::editor().boot_timeout(),
            LifecycleConfig::game().boot_timeout()
        );
        assert_eq!(LifecycleConfig::default(), LifecycleConfig::game());
    }
}
