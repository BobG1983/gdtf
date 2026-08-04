//! Grouped cone and stability tuning structs.

use serde::Deserialize;

use crate::{
    cover::HeightBand,
    tuning::cone::{
        AimConeMult, AimHeightFrac, AimTuPremium, BraceContribution, ConcentrationCoeff,
        EmplacementStabilityBonus, MuzzleForwardOffset, MuzzleHeight, RecoilClimb, SilhouetteTop,
        StabilityCurveCoord, StanceContribution,
    },
};

/// Per-stance stability contribution.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct StanceStability {
    /// Prone contribution.
    pub prone: StanceContribution,
    /// Kneeling contribution.
    pub kneel: StanceContribution,
    /// Standing contribution.
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

/// Minimum cover height band required to brace per stance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BraceMinHeight {
    /// Prone minimum band.
    pub prone: HeightBand,
    /// Kneeling minimum band.
    pub kneel: HeightBand,
    /// Standing minimum band.
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

/// One point on a stability response curve.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct StabilityCurvePoint {
    /// Input score.
    pub score:  StabilityCurveCoord,
    /// Output value.
    pub output: StabilityCurveCoord,
}

/// Ordered list of stability curve points.
#[derive(Debug, Clone, PartialEq, bevy::prelude::Deref, Deserialize)]
#[serde(transparent)]
pub struct StabilityCurve(Vec<StabilityCurvePoint>);

impl StabilityCurve {
    /// Wrap points.
    #[must_use]
    pub const fn new(points: Vec<StabilityCurvePoint>) -> Self {
        Self(points)
    }
}

/// Curves used by cone and recoil calculations.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StabilityCurves {
    /// Cone multiplier vs stability score.
    pub cone_mult:     StabilityCurve,
    /// Recoil growth vs stability score.
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

/// Aimed-fire cone and TU adjustments.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct AimMode {
    /// Cone multiplier when aiming.
    pub cone_mult:  AimConeMult,
    /// Extra TU cost when aiming.
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

/// Concentration base and scale coefficients.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ConcentrationCoeffs {
    /// Base coefficient.
    pub base:  ConcentrationCoeff,
    /// Scale coefficient.
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

/// Per-stance muzzle height.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MuzzleHeights {
    /// Prone height.
    pub prone: MuzzleHeight,
    /// Kneeling height.
    pub kneel: MuzzleHeight,
    /// Standing height.
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

/// Per-stance silhouette top height.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SilhouetteTops {
    /// Prone top.
    pub prone: SilhouetteTop,
    /// Kneeling top.
    pub kneel: SilhouetteTop,
    /// Standing top.
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

/// All cone and stability tuning knobs in one place.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ConeStabilityTuning {
    /// Stance contributions.
    pub stance_stability:            StanceStability,
    /// Brace contribution.
    pub brace_contribution:          BraceContribution,
    /// Emplacement bonus.
    pub emplacement_stability_bonus: EmplacementStabilityBonus,
    /// Brace minimum heights.
    pub brace_min_height:            BraceMinHeight,
    /// Response curves.
    pub stability_curves:            StabilityCurves,
    /// Aim mode adjustments.
    pub aim_mode:                    AimMode,
    /// Recoil climb per shot.
    pub recoil_climb:                RecoilClimb,
    /// Concentration coefficients.
    pub concentration:               ConcentrationCoeffs,
    /// Aim height fraction.
    pub aim_height_frac:             AimHeightFrac,
    /// Muzzle forward offset.
    pub muzzle_forward_offset:       MuzzleForwardOffset,
    /// Muzzle heights by stance.
    pub muzzle_heights:              MuzzleHeights,
    /// Silhouette tops by stance.
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
