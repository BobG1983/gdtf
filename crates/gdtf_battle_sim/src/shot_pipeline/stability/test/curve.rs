//! The score clamp before the curve read + the piecewise-linear `read_curve`
//! form (C5).

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

/// C5 — the score is clamped/normalised to 0–100 BEFORE the curve read: a
/// degenerate over-100 contribution sum does not read off the curve's end or
/// panic. Two wildly over-100 sums whose RAW totals differ (different stance /
/// brace) must still produce the SAME outputs, proving both clamp to the score
/// ceiling (and never a panic / NaN). The over-100 sum is driven by the
/// emplacement term — there is no weapon-points term any more.
#[test]
fn over_100_sum_clamps_and_does_not_run_off_the_curve() {
    let tuning = ConeStabilityTuning::default();
    let wall = faced_cover(HeightBand::High);

    // A wildly over-100 raw sum: prone + braced + a huge emplacement term.
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
    // A DIFFERENT over-100 raw sum (standing, no brace) — also driven over the
    // ceiling by a huge emplacement term, so it too clamps to 100.
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
    // Both clamp to the score ceiling, so the curve reads are identical — the
    // over-100 sums did not run off the curve's end.
    assert_eq!((*over_cone).to_bits(), (*ceil_cone).to_bits());
    assert_eq!((*over_recoil).to_bits(), (*ceil_recoil).to_bits());

    // And the clamped score really is the ceiling (not beyond it).
    assert_eq!(
        (*StabilityScore::clamped(10_000.0)).to_bits(),
        StabilityScore::MAX.to_bits(),
    );
}

/// The piecewise-linear curve read is clamped at both endpoints and
/// interpolates between authored points — the curve *form*. Built from an
/// ARBITRARY two-point curve parsed from a bare-scalar RON fragment (the tuning
/// leaf coords are `#[serde(transparent)]` with private inners, so RON is the
/// in-test build path — and these are arbitrary literals, never shipped values):
/// below the first score returns the first output, above the last returns the
/// last, and a midpoint score returns the midpoint output. An empty curve
/// returns the identity 1.0.
#[test]
fn read_curve_clamps_endpoints_and_interpolates() {
    // Arbitrary two-point curve: score 20→output 2.0, score 60→output 4.0.
    let parsed = ron::from_str::<StabilityCurve>(
        r"[ ( score: 20.0, output: 2.0 ), ( score: 60.0, output: 4.0 ) ]",
    );
    assert!(parsed.is_ok(), "arbitrary curve must parse: {parsed:?}");
    let Ok(curve) = parsed else {
        return;
    };

    // Below the first point's score → first output (left clamp).
    assert_eq!(
        (*read_curve(&curve, StabilityScore::clamped(0.0))).to_bits(),
        2.0_f32.to_bits(),
    );
    // Above the last point's score → last output (right clamp).
    assert_eq!(
        (*read_curve(&curve, StabilityScore::clamped(100.0))).to_bits(),
        4.0_f32.to_bits(),
    );
    // Midpoint score (40 is halfway between 20 and 60) → midpoint output (3.0).
    assert_eq!(
        (*read_curve(&curve, StabilityScore::clamped(40.0))).to_bits(),
        3.0_f32.to_bits(),
    );

    // Empty (degenerate) curve → identity 1.0, never a panic.
    let empty = StabilityCurve::new(Vec::new());
    assert_eq!(
        (*read_curve(&empty, StabilityScore::clamped(50.0))).to_bits(),
        1.0_f32.to_bits(),
    );
}
