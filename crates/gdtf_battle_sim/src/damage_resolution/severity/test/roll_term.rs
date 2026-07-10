//! AC4 — the roll-term bounds (floor-extend, fixed ceiling) + AC6 replay
//! determinism.

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

/// AC4 — the random term is `roll(−L·Luck_defender .. R)`: over a stream of
/// draws, a high-Luck defender's realized roll term goes **negative** (the
/// floor is below 0) AND **never exceeds `R`** (the ceiling is fixed) —
/// confirming NO `R_eff` / NO `R_min` (the lower bound moves, the spread is not
/// merely shrunk). Exercised through the crate-internal [`roll_term`].
#[test]
fn roll_term_floor_extends_below_zero_and_ceiling_is_fixed() {
    let scaling = SeverityScaling::default();
    let ceiling = *scaling.random_spread;
    let mut r = rng();

    let mut saw_negative = false;
    // Many draws so the negative tail is virtually certain to appear.
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

/// AC4 (counterpart) — with **zero** defender Luck the floor is exactly 0, so
/// the roll term is always non-negative and still bounded by `R`. This proves
/// the floor is `−L·Luck_defender` (not a fixed negative), so the lower bound
/// genuinely *moves* with the defender's Luck.
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

/// GTW-644 (C1b) — DRAW-COUNT STABILITY: a DEGENERATE roll-term range (`lo >= hi`)
/// must still consume EXACTLY ONE [`SeverityRng`] draw, so the stream stays aligned
/// with a same-seeded sibling that took a live roll. Two same-seeded streams: one
/// takes a DEGENERATE roll (zero spread + zero defender Luck → `lo = 0 >= hi = 0`),
/// the other a live default-scaling roll; their SUBSEQUENT draws must be identical.
/// RED before the safe-draw fix — the old guard SKIPPED the degenerate draw, so the
/// degenerate stream's next draw came one cursor position early and the same seed
/// produced divergent downstream draws.
#[test]
fn degenerate_roll_term_keeps_the_stream_aligned() {
    let degenerate_scaling = SeverityScaling {
        random_spread: RandomSpread::new(0.0),
        ..SeverityScaling::default()
    };
    let mut through_degenerate = rng();
    let mut reference = rng();

    // lo = −L·0 = 0, hi = R = 0 → lo >= hi: the degenerate range.
    let _ = roll_term(&degenerate_scaling, Luck::new(0.0), &mut through_degenerate);
    // The same-seeded sibling takes a LIVE (non-degenerate) default-scaling roll.
    let _ = roll_term(&SeverityScaling::default(), Luck::new(0.0), &mut reference);

    // Draw-count stability: both streams consumed exactly one draw, so their next
    // draws are identical.
    assert_eq!(
        through_degenerate.next_u64(),
        reference.next_u64(),
        "GTW-644: a degenerate roll_term must consume exactly one draw — the same seed \
         diverges downstream when the degenerate range skips its draw",
    );
}

/// AC6 — seeded determinism: two `roll_severity` SEQUENCES from the same
/// [`BattleSeed`] produce the identical [`Severity`] sequence (replay equality).
/// Drawing a sequence (advancing one shared RNG) proves the stream — not just a
/// single draw — is reproduced.
#[test]
fn same_seed_reproduces_the_severity_sequence() {
    let scaling = SeverityScaling::default();
    let sequence = |seed: u64| {
        let mut r = SeverityRng::from_root(BattleSeed::new(seed));
        // Varying inputs across the sequence so the stream is genuinely walked.
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
