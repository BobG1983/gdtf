use core::time::Duration;
use std::time::Instant;

use cobalt_mcp_server::{
    HostName, LaunchPolicy, SweepClock, SweepDue, SweepEntry, SweepInterval, SweepSchedule,
};

use crate::lifecycle::support::config_sweeping_every;

const BOOT_MS: u64 = 500;

const FAST_EVERY: SweepInterval = SweepInterval::new(Duration::from_millis(11));

const SLOW_EVERY: SweepInterval = SweepInterval::new(Duration::from_millis(23));

fn named(name: &str) -> HostName {
    HostName::new(name.to_owned())
}

fn two_entry_schedule() -> SweepSchedule {
    SweepSchedule::from_entries(vec![
        SweepEntry::new(
            named("alpha"),
            config_sweeping_every(BOOT_MS, FAST_EVERY, LaunchPolicy::Reuse).sweep_interval(),
        ),
        SweepEntry::new(
            named("beta"),
            config_sweeping_every(BOOT_MS, SLOW_EVERY, LaunchPolicy::Reuse).sweep_interval(),
        ),
    ])
}

#[test]
fn each_registered_host_carries_the_interval_its_own_config_reports() {
    let schedule = two_entry_schedule();

    let Some(fast) = schedule.entry(&named("alpha")) else {
        unreachable!("the schedule registers the first host, got: {schedule:?}");
    };
    let Some(slow) = schedule.entry(&named("beta")) else {
        unreachable!("the schedule registers the second host, got: {schedule:?}");
    };
    assert_eq!(
        fast.interval(),
        FAST_EVERY,
        "the entry carries the interval its own config reports"
    );
    assert_eq!(slow.interval(), SLOW_EVERY);
    assert_ne!(
        fast.interval(),
        slow.interval(),
        "each host keeps its own interval instead of sharing one"
    );
}

#[test]
fn a_host_falls_due_on_its_own_interval_and_is_rescheduled_once_swept() {
    let start = Instant::now();
    let mut clock = SweepClock::starting_at(two_entry_schedule(), SweepDue::new(start));

    assert_eq!(
        clock.tick(),
        FAST_EVERY,
        "the loop wakes on the shortest registered interval"
    );
    assert!(
        clock.take_due(SweepDue::new(start)).is_empty(),
        "no host is due before an interval has passed"
    );
    assert_eq!(
        clock.take_due(SweepDue::new(start + *FAST_EVERY)),
        vec![named("alpha")],
        "only the host whose interval has passed is due"
    );
    assert!(
        clock
            .take_due(SweepDue::new(start + *FAST_EVERY))
            .is_empty(),
        "a swept host is rescheduled, so it is not due again at the same instant"
    );
    assert_eq!(
        clock.take_due(SweepDue::new(start + *SLOW_EVERY)),
        vec![named("alpha"), named("beta")],
        "both hosts are due once the longer interval has passed"
    );
}
