use super::{
    super::{PartSeverityMod, Severity, part_severity_mod, roll::severity_score, roll_severity},
    support::*,
};
use crate::{armor::BodyPart, tuning::SeverityScaling};

#[test]
fn severity_is_monotone_in_pen_damage() {
    let scaling = SeverityScaling::default();
    let mut prev = Severity::None;
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

#[test]
fn near_zero_pen_cannot_reach_severe_buckets() {
    let scaling = SeverityScaling::default();

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

#[test]
fn luck_is_directional() {
    let scaling = SeverityScaling::default();
    let base = |shooter: f32, defender: f32| {
        let mut r = rng();
        roll_severity(
            &inputs(8, 0.0, BodyPart::Torso, shooter, defender),
            &scaling,
            &mut r,
        )
    };

    let low_shooter = base(0.0, 0.0);
    let high_shooter = base(6.0, 0.0);
    assert!(
        high_shooter.rank() >= low_shooter.rank(),
        "higher shooter Luck must yield >= severity: {low_shooter:?} -> {high_shooter:?}",
    );

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

    let low_defender = base(0.0, 0.0);
    let high_defender = base(0.0, 6.0);
    assert!(
        high_defender.rank() <= low_defender.rank(),
        "higher defender Luck can only lower or hold severity: {low_defender:?} -> {high_defender:?}",
    );
}

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

#[test]
fn severity_vocabulary_and_part_mod_newtype() {
    assert_eq!(Severity::ALL.len(), 5);
    let ranks: Vec<u8> = Severity::ALL.iter().map(|s| *s.rank()).collect();
    assert_eq!(ranks, vec![0, 1, 2, 3, 4]);
    assert!((*PartSeverityMod::new(3.5) - 3.5).abs() < f32::EPSILON);
    assert!(*part_severity_mod(BodyPart::Head) > *part_severity_mod(BodyPart::Torso));
    assert!(*part_severity_mod(BodyPart::Torso) > *part_severity_mod(BodyPart::LeftLeg));
    assert!(*part_severity_mod(BodyPart::LeftLeg) > *part_severity_mod(BodyPart::LeftArm));
}
