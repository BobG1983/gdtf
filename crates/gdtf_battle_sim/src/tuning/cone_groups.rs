use serde::Deserialize;

use crate::{
    cover::HeightBand,
    tuning::cone::{
        AimConeMult, AimHeightFrac, AimTuPremium, BraceContribution, ConcentrationCoeff,
        EmplacementStabilityBonus, MuzzleForwardOffset, MuzzleHeight, RecoilClimb, SilhouetteTop,
        StabilityCurveCoord, StanceContribution,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct StanceStability {
        pub prone: StanceContribution,
        pub kneel: StanceContribution,
        pub stand: StanceContribution,
}

impl Default for StanceStability {
    fn default() -> Self {
        Self {
            prone: StanceContribution::new(40.0),
            kneel: StanceContribution::new(25.0),
            stand: StanceContribution::new(10.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BraceMinHeight {
        pub prone: HeightBand,
        pub kneel: HeightBand,
        pub stand: HeightBand,
}

impl Default for BraceMinHeight {
    fn default() -> Self {
        Self {
            prone: HeightBand::Low,
            kneel: HeightBand::Mid,
            stand: HeightBand::High,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct StabilityCurvePoint {
        pub score:  StabilityCurveCoord,
        pub output: StabilityCurveCoord,
}

#[derive(Debug, Clone, PartialEq, bevy::prelude::Deref, Deserialize)]
#[serde(transparent)]
pub struct StabilityCurve(Vec<StabilityCurvePoint>);

impl StabilityCurve {
        #[must_use]
    pub const fn new(points: Vec<StabilityCurvePoint>) -> Self {
        Self(points)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StabilityCurves {
            pub cone_mult:     StabilityCurve,
        pub recoil_growth: StabilityCurve,
}

impl Default for StabilityCurves {
    fn default() -> Self {
        Self {
            cone_mult:     StabilityCurve::new(vec![
                StabilityCurvePoint {
                    score:  StabilityCurveCoord::new(0.0),
                    output: StabilityCurveCoord::new(1.0),
                },
                StabilityCurvePoint {
                    score:  StabilityCurveCoord::new(100.0),
                    output: StabilityCurveCoord::new(0.5),
                },
            ]),
            recoil_growth: StabilityCurve::new(vec![
                StabilityCurvePoint {
                    score:  StabilityCurveCoord::new(0.0),
                    output: StabilityCurveCoord::new(1.0),
                },
                StabilityCurvePoint {
                    score:  StabilityCurveCoord::new(100.0),
                    output: StabilityCurveCoord::new(0.25),
                },
            ]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct AimMode {
        pub cone_mult:  AimConeMult,
        pub tu_premium: AimTuPremium,
}

impl Default for AimMode {
    fn default() -> Self {
        Self {
            cone_mult:  AimConeMult::new(0.6),
            tu_premium: AimTuPremium::new(1.5),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ConcentrationCoeffs {
            pub base:  ConcentrationCoeff,
            pub scale: ConcentrationCoeff,
}

impl Default for ConcentrationCoeffs {
    fn default() -> Self {
        Self {
            base:  ConcentrationCoeff::new(1.0),
            scale: ConcentrationCoeff::new(1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MuzzleHeights {
        pub prone: MuzzleHeight,
        pub kneel: MuzzleHeight,
        pub stand: MuzzleHeight,
}

impl Default for MuzzleHeights {
    fn default() -> Self {
        Self {
            prone: MuzzleHeight::new(0.15),
            kneel: MuzzleHeight::new(0.45),
            stand: MuzzleHeight::new(0.8),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SilhouetteTops {
        pub prone: SilhouetteTop,
        pub kneel: SilhouetteTop,
        pub stand: SilhouetteTop,
}

impl Default for SilhouetteTops {
    fn default() -> Self {
        Self {
            prone: SilhouetteTop::new(0.3),
            kneel: SilhouetteTop::new(0.6),
            stand: SilhouetteTop::new(0.95),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ConeStabilityTuning {
        pub stance_stability:            StanceStability,
        pub brace_contribution:          BraceContribution,
                pub emplacement_stability_bonus: EmplacementStabilityBonus,
        pub brace_min_height:            BraceMinHeight,
        pub stability_curves:            StabilityCurves,
        pub aim_mode:                    AimMode,
        pub recoil_climb:                RecoilClimb,
        pub concentration:               ConcentrationCoeffs,
        pub aim_height_frac:             AimHeightFrac,
        pub muzzle_forward_offset:       MuzzleForwardOffset,
        pub muzzle_heights:              MuzzleHeights,
        pub silhouette_tops:             SilhouetteTops,
}

impl Default for ConeStabilityTuning {
    fn default() -> Self {
        Self {
            stance_stability:            StanceStability::default(),
            brace_contribution:          BraceContribution::new(30.0),
            emplacement_stability_bonus: EmplacementStabilityBonus::new(40.0),
            brace_min_height:            BraceMinHeight::default(),
            stability_curves:            StabilityCurves::default(),
            aim_mode:                    AimMode::default(),
            recoil_climb:                RecoilClimb::new(0.01),
            concentration:               ConcentrationCoeffs::default(),
            aim_height_frac:             AimHeightFrac::new(1.0),
            muzzle_forward_offset:       MuzzleForwardOffset::new(0.3),
            muzzle_heights:              MuzzleHeights::default(),
            silhouette_tops:             SilhouetteTops::default(),
        }
    }
}
