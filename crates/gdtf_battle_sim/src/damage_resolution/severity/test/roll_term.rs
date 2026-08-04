use super::{
    super::{roll::roll_term, roll_severity},
    support::*,
};
use crate::{
    armor::BodyPart,
    ganger::Luck,
    rng::{BattleSeed, SeverityRng},
    tuning::{RandomSpread, SeverityScaling},
};

#[test]
fn roll_term_floor_extends_below_zero_and_ceiling_is_fixed() {
    let scaling = SeverityScaling::default();
    let ceiling = *scaling.random_spread;
    let mut r = rng();

    let mut saw_negative = false;
    for _ in 0..1_000 {
        let term = *roll_term(&scaling, Luck::new(6.0), &mut r);
        assert!(
            term <= ceiling,
            "the roll term must never exceed R (the fixed ceiling): {term} > {ceiling}",
        );
        if term < 0.0 {
            saw_negative = true;
        }
    }
    assert!(
        saw_negative,
        "a high-Luck defender's roll term must be able to go negative (floor below 0)",
    );
}

#[test]
fn roll_term_floor_is_zero_without_defender_luck() {
    let scaling = SeverityScaling::default();
    let ceiling = *scaling.random_spread;
    let mut r = rng();
    for _ in 0..1_000 {
        let term = *roll_term(&scaling, Luck::new(0.0), &mut r);
        assert!(
            (0.0..=ceiling).contains(&term),
            "with no defender Luck the term stays in [0, R]: {term}",
        );
    }
}

#[test]
fn degenerate_roll_term_keeps_the_stream_aligned() {
    let degenerate_scaling = SeverityScaling {
        random_spread: RandomSpread::new(0.0),
        ..SeverityScaling::default()
    };
    let mut through_degenerate = rng();
    let mut reference = rng();

    let _ = roll_term(&degenerate_scaling, Luck::new(0.0), &mut through_degenerate);
    let _ = roll_term(&SeverityScaling::default(), Luck::new(0.0), &mut reference);

    assert_eq!(
        through_degenerate.next_u64(),
        reference.next_u64(),
        "a degenerate roll_term must consume exactly one draw — the same seed \
         diverges downstream when the degenerate range skips its draw",
    );
}

#[test]
fn same_seed_reproduces_the_severity_sequence() {
    let scaling = SeverityScaling::default();
    let sequence = |seed: u64| {
        let mut r = SeverityRng::from_root(BattleSeed::new(seed));
        (0..32)
            .map(|i| {
                let pen = i % 12;
                roll_severity(
                    &inputs(pen, 1.0, BodyPart::Torso, 1.0, 2.0),
                    &scaling,
                    &mut r,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        sequence(SEED),
        sequence(SEED),
        "the same battle seed must reproduce the identical severity sequence (replay equality)",
    );
}
