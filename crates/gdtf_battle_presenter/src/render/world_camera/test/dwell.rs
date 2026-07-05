//! Tests of the pure GTW-299 dwell decision + accumulator (mirrors `dwell.rs`).

use super::super::{
    dwell::{DwellElapsed, should_edge_pan_after_dwell},
    tuning::DwellDelaySeconds,
};

/// GTW-299 AC1/AC6 — the PURE dwell decision: `should_edge_pan_after_dwell(accumulated,
/// threshold)` is true exactly when the accumulated linger has REACHED the threshold (inclusive),
/// false while it is still below. Pure, no `App`.
///
/// Pin-discriminating across the three boundary cases — below, AT, and above the threshold — so a
/// regression to a strict `>` (or to ignoring the accumulator) is caught.
#[test]
fn should_edge_pan_after_dwell_gates_on_the_threshold() {
    let threshold = DwellDelaySeconds::new(0.3);

    // BELOW the threshold (a brief graze) — must NOT pan.
    let mut below = DwellElapsed::ZERO;
    below.accumulate(0.2);
    assert!(
        !should_edge_pan_after_dwell(below, threshold),
        "a 0.2 s linger is below the 0.3 s dwell — must not pan",
    );

    // EXACTLY at the threshold — must pan (inclusive gate).
    let mut at = DwellElapsed::ZERO;
    at.accumulate(0.3);
    assert!(
        should_edge_pan_after_dwell(at, threshold),
        "a 0.3 s linger has reached the 0.3 s dwell — must pan",
    );

    // ABOVE the threshold — must pan.
    let mut above = DwellElapsed::ZERO;
    above.accumulate(0.5);
    assert!(
        should_edge_pan_after_dwell(above, threshold),
        "a 0.5 s linger is past the 0.3 s dwell — must pan",
    );

    // A fresh (zero) accumulator never pans.
    assert!(
        !should_edge_pan_after_dwell(DwellElapsed::ZERO, threshold),
        "a zero linger must not pan",
    );
}

/// GTW-299 — `DwellElapsed::reset` snaps the accumulator back to zero, so a cursor that leaves the
/// edge band restarts its dwell from scratch. Pure, no `App`.
#[test]
fn dwell_elapsed_resets_to_zero() {
    let mut elapsed = DwellElapsed::ZERO;
    elapsed.accumulate(0.25);
    assert!(*elapsed > 0.0, "accumulate must grow the linger");
    elapsed.reset();
    assert_eq!(
        elapsed,
        DwellElapsed::ZERO,
        "reset must snap the linger back to zero",
    );
}
