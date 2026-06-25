use bevy::ecs::world::World;

use super::{
    PartSeverityMod, Severity, SeverityInputs, part_severity_mod,
    roll::{roll_term, severity_score},
    roll_severity,
};
use crate::{
    armor::BodyPart,
    ganger::{Luck, Toughness},
    resolve_hit::PenetratingDamage,
    rng::{BattleSeed, SeverityRng},
    tuning::SeverityScaling,
    weapon::FatalBias,
};

/// A fixed seed for the per-test RNG streams — determinism is the property, so
/// the same seed must reproduce the same draws (an arbitrary value, not tuned).
const SEED: u64 = 0xC0FF_EE15;

/// Build a [`SeverityRng`] from the shared fixed seed (a fresh stream per call).
fn rng() -> SeverityRng {
    SeverityRng::from_root(BattleSeed::new(SEED))
}

/// Build a severity-input bundle from arbitrary inputs — a helper so each test
/// varies only the field it exercises. Magnitudes are mechanism inputs, never
/// asserted as values.
fn inputs(
    pen: i32,
    toughness: f32,
    part: BodyPart,
    luck_shooter: f32,
    luck_defender: f32,
) -> SeverityInputs {
    SeverityInputs::new(
        PenetratingDamage::new(pen),
        Toughness::new(toughness),
        part_severity_mod(part),
        FatalBias::new(0.0),
        Luck::new(luck_shooter),
        Luck::new(luck_defender),
    )
}

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
        high_pen_score > low_pen_score,
        "a higher pen_damage must strictly raise the score (pen_term wired in): {high_pen_score} <= {low_pen_score}",
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
        high_shooter_score > low_shooter_score,
        "higher shooter Luck must strictly raise the score (shooter_term wired in): {high_shooter_score} <= {low_shooter_score}",
    );

    // Defender Luck: more ⇒ ≤ bucket (floor extends down; ceiling fixed).
    let low_defender = base(0.0, 0.0);
    let high_defender = base(0.0, 6.0);
    assert!(
        high_defender.rank() <= low_defender.rank(),
        "higher defender Luck can only lower or hold severity: {low_defender:?} -> {high_defender:?}",
    );
}

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
        let term = roll_term(&scaling, Luck::new(6.0), &mut r);
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
        let term = roll_term(&scaling, Luck::new(0.0), &mut r);
        assert!(
            (0.0..=ceiling).contains(&term),
            "with no defender Luck the term stays in [0, R]: {term}",
        );
    }
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
        head_score > leg_score,
        "a head hit must strictly out-score a leg hit at identical seed/inputs (part_mod wired in): {head_score} <= {leg_score}",
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

/// AC7 (mechanism) / no-bare-types — [`Severity`] is a named enum with the five
/// buckets in ascending rank, and [`PartSeverityMod`] derefs to its inner
/// `f32`. Pins the vocabulary + the Deref mechanism (arbitrary values), never a
/// tuning magnitude.
#[test]
fn severity_vocabulary_and_part_mod_newtype() {
    assert_eq!(Severity::ALL.len(), 5);
    // Ascending ranks, distinct per bucket.
    let ranks: Vec<u8> = Severity::ALL.iter().map(|s| s.rank()).collect();
    assert_eq!(ranks, vec![0, 1, 2, 3, 4]);
    // The part-mod newtype derefs to its inner f32 (arbitrary literal).
    assert!((*PartSeverityMod::new(3.5) - 3.5).abs() < f32::EPSILON);
    // The placeholder per-part ordering: head > torso > legs >= arms-or-below.
    assert!(*part_severity_mod(BodyPart::Head) > *part_severity_mod(BodyPart::Torso));
    assert!(*part_severity_mod(BodyPart::Torso) > *part_severity_mod(BodyPart::LeftLeg));
    assert!(*part_severity_mod(BodyPart::LeftLeg) > *part_severity_mod(BodyPart::LeftArm));
}

/// Contract — the defender's Toughness and both gangers' Luck are **sourced off
/// the entity** (the E3.0 components), then fed to `roll_severity`. A bare
/// `World` spawns a shooter and a defender each carrying the attribute
/// components, a `Query`/`World` access reads them back, and they drive the
/// roll — proving the stats come off the entity, not bare literals.
#[test]
fn stats_are_sourced_off_the_entity() {
    let scaling = SeverityScaling::default();

    let mut world = World::new();
    let shooter = world.spawn(Luck::new(2.0)).id();
    let defender = world.spawn((Toughness::new(3.0), Luck::new(4.0))).id();

    // Read the shooter's Luck off its entity.
    let mut shooter_q = world.query::<&Luck>();
    let shooter_read = shooter_q.get(&world, shooter);
    assert!(
        shooter_read.is_ok(),
        "shooter Luck must be queryable off the entity",
    );
    let Ok(&luck_shooter) = shooter_read else {
        return;
    };

    // Read the defender's Toughness + Luck off its entity.
    let mut defender_q = world.query::<(&Toughness, &Luck)>();
    let defender_read = defender_q.get(&world, defender);
    assert!(
        defender_read.is_ok(),
        "defender Toughness + Luck must be queryable off the entity",
    );
    let Ok((&toughness, &luck_defender)) = defender_read else {
        return;
    };

    // Feed the entity-sourced stats into the roll — the real read shape.
    let bundle = SeverityInputs::new(
        PenetratingDamage::new(10),
        toughness,
        part_severity_mod(BodyPart::Torso),
        FatalBias::new(0.0),
        luck_shooter,
        luck_defender,
    );
    let mut r = rng();
    let got = roll_severity(&bundle, &scaling, &mut r);
    // Value-agnostic: a real Severity came back from entity-sourced stats.
    assert!(
        Severity::ALL.contains(&got),
        "roll_severity must return a Severity from entity-sourced stats: {got:?}",
    );
}
