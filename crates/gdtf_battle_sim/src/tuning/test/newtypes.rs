//! The tuning-newtype Deref mechanism pins (arbitrary literals, never shipped
//! magnitudes).

use super::super::*;
use crate::cover::HeightBand;

/// Each tuning newtype wraps the right inner type and its derived [`Deref`]
/// reaches that inner value (C9/C10/C11 mandate a derived `Deref` on every
/// tuning newtype; this exercises that surface so dropping the derive would
/// fail a test).
///
/// Built from **arbitrary** literals, never the shipped/default magnitudes:
/// this pins the Deref *target type and mechanism*, not a balance value, so
/// it stays non-brittle against a tuning edit. The `f32` newtypes are
/// compared by bit pattern (the literals are exactly representable, so this
/// is an exact integer equality — no `float_cmp` lint, no epsilon).
#[test]
fn tuning_newtypes_wrap_inner_and_deref() {
    // Each f32 newtype: deref reaches the inner f32 (bit-exact arbitrary
    // value, not the default).
    assert_eq!((*BandEdge::new(5.0)).to_bits(), 5.0_f32.to_bits());
    assert_eq!((*PenDamageScale::new(2.5)).to_bits(), 2.5_f32.to_bits());
    assert_eq!(
        (*ToughnessMitigation::new(3.5)).to_bits(),
        3.5_f32.to_bits()
    );
    assert_eq!((*ShooterLuckScale::new(4.5)).to_bits(), 4.5_f32.to_bits());
    assert_eq!((*DefenderLuckScale::new(6.5)).to_bits(), 6.5_f32.to_bits());
    assert_eq!((*RandomSpread::new(7.5)).to_bits(), 7.5_f32.to_bits());
    assert_eq!((*SeverityEdge::new(8.5)).to_bits(), 8.5_f32.to_bits());
    // The u16 newtype: deref reaches the inner u16 (arbitrary value).
    assert_eq!(*BodyPartWeight::new(3), 3u16);
    // The u8 bleed-out rate: deref reaches the inner u8 (arbitrary value, the
    // mechanism not the shipped magnitude).
    assert_eq!(*BleedRate::new(4), 4u8);
    // GTW-242 — the firing-arc f32 newtype: deref reaches the inner degrees (bit-exact
    // arbitrary value, never the 120° default — the Deref mechanism, not a magnitude).
    assert_eq!((*FiringArc::new(75.0)).to_bits(), 75.0_f32.to_bits());
    // GTW-338 — the view-range u16 newtype: deref reaches the inner Chebyshev cell count
    // (arbitrary value, never the 14 default — the Deref mechanism, not a magnitude).
    assert_eq!(*ViewRange::new(9), 9u16);
    // GTW-338 — the explored-dim f32 newtype: deref reaches the inner modulate factor
    // (bit-exact arbitrary value, never the 0.55 default — the Deref mechanism only).
    assert_eq!((*ExploredDim::new(0.25)).to_bits(), 0.25_f32.to_bits());
    // GTW-349 — the per-link traversal u8 newtype: deref reaches the inner TU count
    // (arbitrary value, never the default — the Deref mechanism, not a magnitude).
    assert_eq!(*LinkTu::new(5), 5u8);
}

/// C3 — every E2.1 cone/stability/recoil/aim extension leaf wraps the right
/// inner type and its derived [`Deref`] reaches it. Built from **arbitrary**
/// literals (never the shipped/default magnitudes), so this pins the Deref
/// mechanism + target type, not a balance value (bit-exact f32 equality on
/// exactly-representable literals — no `float_cmp` lint).
#[test]
fn cone_stability_newtypes_wrap_inner_and_deref() {
    assert_eq!(
        (*StanceContribution::new(11.0)).to_bits(),
        11.0_f32.to_bits()
    );
    assert_eq!(
        (*BraceContribution::new(22.0)).to_bits(),
        22.0_f32.to_bits()
    );
    assert_eq!(
        (*StabilityCurveCoord::new(33.0)).to_bits(),
        33.0_f32.to_bits()
    );
    assert_eq!((*AimConeMult::new(0.25)).to_bits(), 0.25_f32.to_bits());
    assert_eq!((*AimTuPremium::new(1.25)).to_bits(), 1.25_f32.to_bits());
    assert_eq!((*RecoilClimb::new(0.5)).to_bits(), 0.5_f32.to_bits());
    assert_eq!((*ConcentrationCoeff::new(2.5)).to_bits(), 2.5_f32.to_bits());
    assert_eq!((*AimHeightFrac::new(0.75)).to_bits(), 0.75_f32.to_bits());
    assert_eq!(
        (*MuzzleForwardOffset::new(0.125)).to_bits(),
        0.125_f32.to_bits()
    );
    assert_eq!((*MuzzleHeight::new(0.625)).to_bits(), 0.625_f32.to_bits());
    assert_eq!((*SilhouetteTop::new(0.875)).to_bits(), 0.875_f32.to_bits());

    // The curve newtype derefs to its inner Vec (arbitrary one-point shape).
    let curve = StabilityCurve::new(vec![StabilityCurvePoint {
        score:  StabilityCurveCoord::new(50.0),
        output: StabilityCurveCoord::new(0.7),
    }]);
    assert_eq!(curve.len(), 1);
    assert_eq!((*curve[0].output).to_bits(), 0.7_f32.to_bits());

    // The brace gate carries a HeightBand per stance (not a magnitude).
    let gate = BraceMinHeight {
        prone: HeightBand::Low,
        kneel: HeightBand::Mid,
        stand: HeightBand::High,
    };
    assert_eq!(gate.stand, HeightBand::High);
}
