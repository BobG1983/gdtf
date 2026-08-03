use super::super::{
    dwell::{DwellElapsed, should_edge_pan_after_dwell},
    tuning::DwellDelaySeconds,
};

#[test]
fn should_edge_pan_after_dwell_gates_on_the_threshold() {
    let threshold = DwellDelaySeconds::new(0.3);

    let mut below = DwellElapsed::ZERO;
    below.accumulate(0.2);
    assert!(
        !should_edge_pan_after_dwell(below, threshold),
        "a 0.2 s linger is below the 0.3 s dwell — must not pan",
    );

    let mut at = DwellElapsed::ZERO;
    at.accumulate(0.3);
    assert!(
        should_edge_pan_after_dwell(at, threshold),
        "a 0.3 s linger has reached the 0.3 s dwell — must pan",
    );

    let mut above = DwellElapsed::ZERO;
    above.accumulate(0.5);
    assert!(
        should_edge_pan_after_dwell(above, threshold),
        "a 0.5 s linger is past the 0.3 s dwell — must pan",
    );

    assert!(
        !should_edge_pan_after_dwell(DwellElapsed::ZERO, threshold),
        "a zero linger must not pan",
    );
}

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
