//! How often each host's recorded child is checked for liveness.

use core::ops::Deref;
use std::time::Instant;

use super::{config::LifecycleConfig, values::SweepInterval};
use crate::hosts::{HostName, HostRegistry};

/// When a host's next sweep falls due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepDue(Instant);

impl SweepDue {
    /// Wrap an instant.
    #[must_use]
    pub const fn new(at: Instant) -> Self {
        Self(at)
    }
}

impl Deref for SweepDue {
    type Target = Instant;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// One host's registered sweep interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SweepEntry {
    host:     HostName,
    interval: SweepInterval,
}

impl SweepEntry {
    /// Register `host` to be swept every `interval`.
    #[must_use]
    pub const fn new(host: HostName, interval: SweepInterval) -> Self {
        Self { host, interval }
    }

    /// Host this entry sweeps.
    #[must_use]
    pub const fn host(&self) -> &HostName {
        &self.host
    }

    /// How often that host is swept.
    #[must_use]
    pub const fn interval(&self) -> SweepInterval {
        self.interval
    }
}

/// The hosts to sweep and how often, one entry each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SweepSchedule {
    entries: Vec<SweepEntry>,
}

impl SweepSchedule {
    /// Register every host, each with the interval its own config reports.
    #[must_use]
    pub fn from_registry(registry: &HostRegistry) -> Self {
        Self {
            entries: registry
                .hosts()
                .iter()
                .map(|host| {
                    SweepEntry::new(
                        host.name().clone(),
                        host.lifecycle_config().sweep_interval(),
                    )
                })
                .collect(),
        }
    }

    /// Register these entries as written.
    #[must_use]
    pub const fn from_entries(entries: Vec<SweepEntry>) -> Self {
        Self { entries }
    }

    /// Registered entry for `host`, if there is one.
    #[must_use]
    pub fn entry(&self, host: &HostName) -> Option<&SweepEntry> {
        self.entries.iter().find(|entry| entry.host() == host)
    }

    /// Shortest registered interval, if anything is registered.
    #[must_use]
    pub fn shortest_interval(&self) -> Option<SweepInterval> {
        self.entries
            .iter()
            .map(SweepEntry::interval)
            .min_by_key(|interval| **interval)
    }
}

/// Tracks which hosts are due a sweep.
pub struct SweepClock {
    schedule: SweepSchedule,
    due:      Vec<SweepDue>,
    tick:     SweepInterval,
}

impl SweepClock {
    /// Start every registered host's countdown from now.
    #[must_use]
    pub fn started(schedule: SweepSchedule) -> Self {
        Self::starting_at(schedule, SweepDue::new(Instant::now()))
    }

    /// Start every registered host's countdown from `now`.
    #[must_use]
    pub fn starting_at(schedule: SweepSchedule, now: SweepDue) -> Self {
        let tick = schedule
            .shortest_interval()
            .unwrap_or_else(|| LifecycleConfig::default().sweep_interval());
        let due = schedule
            .entries
            .iter()
            .map(|entry| SweepDue::new(*now + *entry.interval()))
            .collect();
        Self {
            schedule,
            due,
            tick,
        }
    }

    /// How long the caller may wait before asking again.
    #[must_use]
    pub const fn tick(&self) -> SweepInterval {
        self.tick
    }

    /// Hosts due a sweep at `now`, each rescheduled by its own interval.
    pub fn take_due(&mut self, now: SweepDue) -> Vec<HostName> {
        let mut ready = Vec::new();
        for (entry, due) in self.schedule.entries.iter().zip(self.due.iter_mut()) {
            if *now < **due {
                continue;
            }
            *due = SweepDue::new(*now + *entry.interval());
            ready.push(entry.host().clone());
        }
        ready
    }
}
