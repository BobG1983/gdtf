use core::time::Duration;
use std::time::Instant;

use gdtf_qa_mcp::{LaunchPolicy, QaHost, SweepClock, SweepDue, SweepInterval, SweepSchedule};

use crate::support::config_sweeping_every;

const BOOT_MS: u64 = 500;

const GAME_EVERY: SweepInterval = SweepInterval::new(Duration::from_millis(11));

const EDITOR_EVERY: SweepInterval = SweepInterval::new(Duration::from_millis(23));

#[test]
fn each_registered_host_carries_the_interval_its_own_config_reports() {
    let game = config_sweeping_every(BOOT_MS, GAME_EVERY, LaunchPolicy::Reuse);
    let editor = config_sweeping_every(BOOT_MS, EDITOR_EVERY, LaunchPolicy::Reuse);

    let schedule = SweepSchedule::from_configs(game, editor);

    let Some(game_entry) = schedule.entry(QaHost::Game) else {
        unreachable!("the schedule registers the game host, got: {schedule:?}");
    };
    let Some(editor_entry) = schedule.entry(QaHost::Editor) else {
        unreachable!("the schedule registers the editor host, got: {schedule:?}");
    };
    assert_eq!(
        game_entry.interval(),
        game.sweep_interval(),
        "the game entry carries the interval the game config reports"
    );
    assert_eq!(
        editor_entry.interval(),
        editor.sweep_interval(),
        "the editor entry carries the interval the editor config reports"
    );
    assert_ne!(
        game_entry.interval(),
        editor_entry.interval(),
        "each host keeps its own interval instead of sharing one"
    );
}

#[test]
fn a_host_falls_due_on_its_own_interval_and_is_rescheduled_once_swept() {
    let start = Instant::now();
    let schedule = SweepSchedule::from_configs(
        config_sweeping_every(BOOT_MS, GAME_EVERY, LaunchPolicy::Reuse),
        config_sweeping_every(BOOT_MS, EDITOR_EVERY, LaunchPolicy::Reuse),
    );
    let mut clock = SweepClock::starting_at(schedule, SweepDue::new(start));

    assert_eq!(
        clock.tick(),
        GAME_EVERY,
        "the loop wakes on the shortest registered interval"
    );
    assert!(
        clock.take_due(SweepDue::new(start)).is_empty(),
        "no host is due before an interval has passed"
    );
    assert_eq!(
        clock.take_due(SweepDue::new(start + *GAME_EVERY)),
        vec![QaHost::Game],
        "only the host whose interval has passed is due"
    );
    assert!(
        clock
            .take_due(SweepDue::new(start + *GAME_EVERY))
            .is_empty(),
        "a swept host is rescheduled, so it is not due again at the same instant"
    );
    assert_eq!(
        clock.take_due(SweepDue::new(start + *EDITOR_EVERY)),
        vec![QaHost::Game, QaHost::Editor],
        "both hosts are due once the longer interval has passed"
    );
}
