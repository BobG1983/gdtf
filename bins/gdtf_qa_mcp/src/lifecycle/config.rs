use core::time::Duration;

use super::values::{BootTimeout, KillGrace, PollInterval, ProbeTimeout};

const GAME_BOOT_TIMEOUT: BootTimeout = BootTimeout::new(Duration::from_secs(180));

const EDITOR_BOOT_TIMEOUT: BootTimeout = BootTimeout::new(Duration::from_secs(600));

const DEFAULT_POLL_INTERVAL: PollInterval = PollInterval::new(Duration::from_millis(250));

const DEFAULT_KILL_GRACE: KillGrace = KillGrace::new(Duration::from_secs(5));

const DEFAULT_PROBE_TIMEOUT: ProbeTimeout = ProbeTimeout::new(Duration::from_millis(500));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleConfig {
        boot_timeout:  BootTimeout,
        poll_interval: PollInterval,
        kill_grace:    KillGrace,
        probe_timeout: ProbeTimeout,
}

impl LifecycleConfig {
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

        #[must_use]
    pub const fn boot_timeout(&self) -> BootTimeout {
        self.boot_timeout
    }

        #[must_use]
    pub const fn poll_interval(&self) -> PollInterval {
        self.poll_interval
    }

        #[must_use]
    pub const fn kill_grace(&self) -> KillGrace {
        self.kill_grace
    }

        #[must_use]
    pub const fn probe_timeout(&self) -> ProbeTimeout {
        self.probe_timeout
    }

        #[must_use]
    pub const fn game() -> Self {
        Self::new(
            GAME_BOOT_TIMEOUT,
            DEFAULT_POLL_INTERVAL,
            DEFAULT_KILL_GRACE,
            DEFAULT_PROBE_TIMEOUT,
        )
    }

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
    fn the_editor_waits_longer_for_readiness_than_the_game() {
        assert!(
            *LifecycleConfig::editor().boot_timeout() > *LifecycleConfig::game().boot_timeout()
        );
        assert_eq!(LifecycleConfig::default(), LifecycleConfig::game());
    }
}
