use core::time::Duration;
use std::time::Instant;

use crate::lifecycle::{manager::host::boot_deadline, values::BootTimeout};

#[test]
fn a_boot_wait_that_never_runs_out_has_no_deadline() {
    assert_eq!(
        boot_deadline(Instant::now(), BootTimeout::new(Duration::MAX)),
        None,
        "the longest boot wait must answer no deadline rather than overflow the instant",
    );
}

#[test]
fn a_finite_boot_wait_has_a_deadline_ahead_of_now() {
    let now = Instant::now();

    let deadline = boot_deadline(now, BootTimeout::new(Duration::from_millis(1)));

    assert!(
        deadline.is_some_and(|at| *at > now),
        "a finite boot wait must answer a deadline later than the instant it started from",
    );
}
