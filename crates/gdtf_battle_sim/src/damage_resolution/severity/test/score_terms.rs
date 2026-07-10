//! Severity score term wiring + monotone bucket relations (AC1 / AC2 / AC3 /
//! AC5 / AC7).

use super::{
    super::{PartSeverityMod, Severity, part_severity_mod, roll::severity_score, roll_severity},
    support::*,
};
use crate::{armor::BodyPart, tuning::SeverityScaling};

/// AC1 — bucketing is monotone in the score: with a **fixed seed** (so the
/// random term is identical across calls), rising `pen_damage` yields a
/// non-decreasing [`Severity`] rank. Asserts the relation, never an edge value.
#[test]
fn severity_is_monotone_in_pen_damage() {
    let scaling = SeverityScaling::default();
    let mut prev = Severity::None;
    // A rising pen sweep; each call gets a FRESH same-seeded RNG so the roll
    // term is held identical and only pen moves the score.
    for pen in [0, 2, 5, 10, 20, 40, 80] {
        let mut r = rng();
        let got = roll_severity(
            &inputs(pen, 0.0, BodyPart::Torso, 0.0, 0.0),
            &scaling,
            &mut r,
        );
        assert!(
            got.rank() >= prev.rank(),
            "severity must not decrease as pen rises: {prev:?} -> {got:?} at pen {pen}",
        );
        prev = got;
    }
}

/// AC2 — penetration-gated, two ways (mechanism, never a magnitude):
///
/// 1. The `< e0` graze branch is reachable for a near-zero-pen hit: with pen 0
///    and the score driven below `e0` by a dominating Toughness mitigation
///    (`−k·Toughness` outweighs the bounded `[lo, R]` roll), the bucket is
///    [`Severity::None`] regardless of the draw.
/// 2. A near-zero-pen hit cannot reach the **severe** buckets that a high-pen
///    hit reaches: holding the seed and all else, the near-zero-pen bucket is
///    `≤` the high-pen bucket — pen genuinely gates the climb.
#[test]
fn near_zero_pen_cannot_reach_severe_buckets() {
    let scaling = SeverityScaling::default();

    // (1) None is reachable: a huge Toughness drives −k·Toughness below the
    // roll ceiling so the score is negative for ANY draw — a graze. The
    // Toughness magnitude is a test fixture (a dominating value), not a pinned
    // tuning number; we assert the branch, not a score.
    let mut graze_rng = rng();
    let graze = roll_severity(
        &inputs(0, 1.0e9, BodyPart::LeftLeg, 0.0, 0.0),
        &scaling,
        &mut graze_rng,
    );
    assert_eq!(
        graze,
        Severity::None,
        "a near-zero-pen hit whose score is driven below e0 must be a graze (None)",
    );

    // (2) Pen gates the climb: at the SAME seed, a near-zero-pen hit buckets
    // no higher than a high-pen hit (it cannot reach the severe buckets the
    // high-pen hit can). Identical inputs but pen — a relation, not a value.
    let mut low_rng = rng();
    let low_pen = roll_severity(
        &inputs(0, 0.0, BodyPart::Torso, 0.0, 0.0),
        &scaling,
        &mut low_rng,
    );
    let mut high_rng = rng();
    let high_pen = roll_severity(
        &inputs(100, 0.0, BodyPart::Torso, 0.0, 0.0),
        &scaling,
        &mut high_rng,
    );
    assert!(
        low_pen.rank() <= high_pen.rank(),
        "a near-zero-pen hit must not out-bucket a high-pen hit: {low_pen:?} > {high_pen:?}",
    );

    // STRICT (pins pen_term into the score): at a fixed same-seed RNG the roll
    // term is byte-identical, so high − low pen score is deterministically
    // j·Δpen = j·100 (> 0 for the default scaling). `>` fails iff `pen_term`
    // is reverted out of the score — the non-strict bucket relation above is
    // satisfied by a pen-constant (reverted) score, this raw-score check is not.
    let mut low_score_rng = rng();
    let low_pen_score = severity_score(
        &inputs(0, 0.0, BodyPart::Torso, 0.0, 0.0),
        &scaling,
        &mut low_score_rng,
    );
    let mut high_score_rng = rng();
    let high_pen_score = severity_score(
        &inputs(100, 0.0, BodyPart::Torso, 0.0, 0.0),
        &scaling,
        &mut high_score_rng,
    );
    assert!(
        *high_pen_score > *low_pen_score,
        "a higher pen_damage must strictly raise the score (pen_term wired in): {} <= {}",
        *high_pen_score,
        *low_pen_score,
    );
}

/// AC3 — directional Luck: holding the seed, a higher **shooter** Luck yields a
/// `≥` bucket (pushes the score up) and a higher **defender** Luck yields a `≤`
/// bucket (extends the floor down, ceiling fixed). Both directions asserted as
/// relations with a same-seeded RNG per call (so only Luck moves).
#[test]
fn luck_is_directional() {
    let scaling = SeverityScaling::default();
    // A mid-range pen so there's room to move up or down between buckets.
    let base = |shooter: f32, defender: f32| {
        let mut r = rng();
        roll_severity(
            &inputs(8, 0.0, BodyPart::Torso, shooter, defender),
            &scaling,
            &mut r,
        )
    };

    // Shooter Luck: more ⇒ ≥ bucket.
    let low_shooter = base(0.0, 0.0);
    let high_shooter = base(6.0, 0.0);
    assert!(
        high_shooter.rank() >= low_shooter.rank(),
        "higher shooter Luck must yield >= severity: {low_shooter:?} -> {high_shooter:?}",
    );

    // STRICT (pins shooter_term into the score): at a fixed same-seed RNG the
    // roll term is byte-identical, so high − low shooter score is
    // deterministically I·Δluck = I·6.0 (> 0 for the default scaling). `>`
    // fails iff `shooter_term` is reverted out of the score — the bucket
    // relation above is vacuous on revert (equal scores), this is not.
    let mut low_shooter_rng = rng();
    let low_shooter_score = severity_score(
        &inputs(8, 0.0, BodyPart::Torso, 0.0, 0.0),
        &scaling,
        &mut low_shooter_rng,
    );
    let mut high_shooter_rng = rng();
    let high_shooter_score = severity_score(
        &inputs(8, 0.0, BodyPart::Torso, 6.0, 0.0),
        &scaling,
        &mut high_shooter_rng,
    );
    assert!(
        *high_shooter_score > *low_shooter_score,
        "higher shooter Luck must strictly raise the score (shooter_term wired in): {} <= {}",
        *high_shooter_score,
        *low_shooter_score,
    );

    // Defender Luck: more ⇒ ≤ bucket (floor extends down; ceiling fixed).
    let low_defender = base(0.0, 0.0);
    let high_defender = base(0.0, 6.0);
    assert!(
        high_defender.rank() <= low_defender.rank(),
        "higher defender Luck can only lower or hold severity: {low_defender:?} -> {high_defender:?}",
    );
}

/// AC5 — `part_mod` is location-dependent: at an identical seed and inputs, a
/// **head** hit yields a `≥` score (and bucket) than a **leg** hit (head +12 vs
/// legs 0). Asserts the raw score relation (the part mod is wired in) AND the
/// bucket relation. Same-seeded RNG per call so only the part moves.
#[test]
fn head_hit_outscores_leg_hit() {
    let scaling = SeverityScaling::default();

    let mut head_rng = rng();
    let head_score = severity_score(
        &inputs(8, 0.0, BodyPart::Head, 0.0, 0.0),
        &scaling,
        &mut head_rng,
    );
    let mut leg_rng = rng();
    let leg_score = severity_score(
        &inputs(8, 0.0, BodyPart::LeftLeg, 0.0, 0.0),
        &scaling,
        &mut leg_rng,
    );
    // STRICT: at a fixed same-seed RNG the roll term is byte-identical, so
    // head_score − leg_score is deterministically +12.0 (head +12 vs leg 0).
    // `>` (not `>=`) is therefore always correct here AND fails iff the
    // `+ part_mod` term is reverted out of the score — pinning the wiring.
    assert!(
        *head_score > *leg_score,
        "a head hit must strictly out-score a leg hit at identical seed/inputs (part_mod wired in): {} <= {}",
        *head_score,
        *leg_score,
    );

    let mut head_b = rng();
    let head_sev = roll_severity(
        &inputs(8, 0.0, BodyPart::Head, 0.0, 0.0),
        &scaling,
        &mut head_b,
    );
    let mut leg_b = rng();
    let leg_sev = roll_severity(
        &inputs(8, 0.0, BodyPart::LeftLeg, 0.0, 0.0),
        &scaling,
        &mut leg_b,
    );
    assert!(
        head_sev.rank() >= leg_sev.rank(),
        "a head hit must yield >= severity than a leg hit: {leg_sev:?} -> {head_sev:?}",
    );
}

/// AC7 (mechanism) / no-bare-types — [`Severity`] is a named enum with the five
/// buckets in ascending rank, and [`PartSeverityMod`] derefs to its inner
/// `f32`. Pins the vocabulary + the Deref mechanism (arbitrary values), never a
/// tuning magnitude.
#[test]
fn severity_vocabulary_and_part_mod_newtype() {
    assert_eq!(Severity::ALL.len(), 5);
    // Ascending ranks, distinct per bucket.
    let ranks: Vec<u8> = Severity::ALL.iter().map(|s| *s.rank()).collect();
    assert_eq!(ranks, vec![0, 1, 2, 3, 4]);
    // The part-mod newtype derefs to its inner f32 (arbitrary literal).
    assert!((*PartSeverityMod::new(3.5) - 3.5).abs() < f32::EPSILON);
    // The placeholder per-part ordering: head > torso > legs >= arms-or-below.
    assert!(*part_severity_mod(BodyPart::Head) > *part_severity_mod(BodyPart::Torso));
    assert!(*part_severity_mod(BodyPart::Torso) > *part_severity_mod(BodyPart::LeftLeg));
    assert!(*part_severity_mod(BodyPart::LeftLeg) > *part_severity_mod(BodyPart::LeftArm));
}
