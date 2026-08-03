use super::super::*;
use crate::cover::HeightBand;

#[test]
fn tuning_newtypes_wrap_inner_and_deref() {
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
    assert_eq!(*BodyPartWeight::new(3), 3u16);
    assert_eq!(*BleedRate::new(4), 4u8);
    assert_eq!((*FiringArc::new(75.0)).to_bits(), 75.0_f32.to_bits());
    assert_eq!(*ViewRange::new(9), 9u16);
    assert_eq!((*ExploredDim::new(0.25)).to_bits(), 0.25_f32.to_bits());
    assert_eq!(*LinkTu::new(5), 5u8);
}

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

    let curve = StabilityCurve::new(vec![StabilityCurvePoint {
        score:  StabilityCurveCoord::new(50.0),
        output: StabilityCurveCoord::new(0.7),
    }]);
    assert_eq!(curve.len(), 1);
    assert_eq!((*curve[0].output).to_bits(), 0.7_f32.to_bits());

    let gate = BraceMinHeight {
        prone: HeightBand::Low,
        kneel: HeightBand::Mid,
        stand: HeightBand::High,
    };
    assert_eq!(gate.stand, HeightBand::High);
}
