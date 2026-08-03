use super::support::*;
use crate::{
    cover::HeightBand,
    ganger::{Stance, StanceKind},
    stability::{
        curve::read_curve,
        score::stability,
        types::{EmplacementStability, StabilityScore, StabilityTerms, SuppressionStability},
    },
    tuning::{ConeStabilityTuning, StabilityCurve},
};

#[test]
fn over_100_sum_clamps_and_does_not_run_off_the_curve() {
    let tuning = ConeStabilityTuning::default();
    let wall = faced_cover(HeightBand::High);

    let (over_cone, over_recoil) = stability(
        StabilityTerms {
            emplacement: EmplacementStability::new(10_000.0),
            ..StabilityTerms::default()
        },
        Stance::new(StanceKind::Prone),
        Some(&wall),
        SuppressionStability::none(),
        &tuning,
    );
    let (ceil_cone, ceil_recoil) = stability(
        StabilityTerms {
            emplacement: EmplacementStability::new(10_000.0),
            ..StabilityTerms::default()
        },
        Stance::new(StanceKind::Standing),
        None,
        SuppressionStability::none(),
        &tuning,
    );

    assert!((*over_cone).is_finite() && (*over_recoil).is_finite());
    assert_eq!((*over_cone).to_bits(), (*ceil_cone).to_bits());
    assert_eq!((*over_recoil).to_bits(), (*ceil_recoil).to_bits());

    assert_eq!(
        (*StabilityScore::clamped(10_000.0)).to_bits(),
        StabilityScore::MAX.to_bits(),
    );
}

/// leaf coords are `#[serde(transparent)]` with private inners, so RON is the
#[test]
fn read_curve_clamps_endpoints_and_interpolates() {
    let parsed = ron::from_str::<StabilityCurve>(
        r"[ ( score: 20.0, output: 2.0 ), ( score: 60.0, output: 4.0 ) ]",
    );
    assert!(parsed.is_ok(), "arbitrary curve must parse: {parsed:?}");
    let Ok(curve) = parsed else {
        return;
    };

    assert_eq!(
        (*read_curve(&curve, StabilityScore::clamped(0.0))).to_bits(),
        2.0_f32.to_bits(),
    );
    assert_eq!(
        (*read_curve(&curve, StabilityScore::clamped(100.0))).to_bits(),
        4.0_f32.to_bits(),
    );
    assert_eq!(
        (*read_curve(&curve, StabilityScore::clamped(40.0))).to_bits(),
        3.0_f32.to_bits(),
    );

    let empty = StabilityCurve::new(Vec::new());
    assert_eq!(
        (*read_curve(&empty, StabilityScore::clamped(50.0))).to_bits(),
        1.0_f32.to_bits(),
    );
}
