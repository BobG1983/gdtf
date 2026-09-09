use core::time::Duration;
use std::time::Instant;

use super::grace_deadline;
use crate::lifecycle::values::KillGrace;

#[test]
fn a_grace_that_never_runs_out_has_no_deadline() {
    assert_eq!(
        grace_deadline(Instant::now(), KillGrace::new(Duration::MAX)),
        None,
        "the longest grace must answer no deadline rather than overflow the instant",
    );
}

#[test]
fn a_finite_grace_has_a_deadline_ahead_of_now() {
    let now = Instant::now();

    let deadline = grace_deadline(now, KillGrace::new(Duration::from_millis(1)));

    assert!(
        deadline.is_some_and(|at| *at > now),
        "a finite grace must answer a deadline later than the instant it started from",
    );
}
