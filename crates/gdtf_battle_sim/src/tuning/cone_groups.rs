//! The §1 cone / stability / recoil / aim **grouping structs** + the
//! [`ConeStabilityTuning`] bundle (resolution.md §1a / §1b). The leaf coefficient
//! newtypes these compose live in [`cone`](crate::tuning::cone).

use serde::Deserialize;

use crate::{
    cover::HeightBand,
    tuning::cone::{
        AimConeMult, AimHeightFrac, AimTuPremium, BraceContribution, ConcentrationCoeff,
        EmplacementStabilityBonus, MuzzleForwardOffset, MuzzleHeight, RecoilClimb,
        SightStabilityBonus, SilhouetteTop, StabilityCurveCoord, StanceContribution,
    },
};

/// The per-stance **stability contributions** — the points each stance adds to the
/// 0–100 stability score (resolution.md §1a: prone 40 / kneel 25 / stand 10).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct StanceStability {
    /// Prone — the steadiest stance's contribution (doc default 40).
    pub prone: StanceContribution,
    /// Kneeling — the middle contribution (doc default 25).
    pub kneel: StanceContribution,
    /// Standing — the least steady contribution (doc default 10).
    pub stand: StanceContribution,
}

impl Default for StanceStability {
    fn default() -> Self {
        // Stance contributions from resolution.md §1a (prone 40 / kneel 25 /
        // stand 10) — tunable balance data, not pinned by a value test.
        Self {
            prone: StanceContribution::new(40.0),
            kneel: StanceContribution::new(25.0),
            stand: StanceContribution::new(10.0),
        }
    }
}

/// The per-stance **brace min-height gate** — the minimum cover [`HeightBand`] a
/// faced cell must reach for each stance's automatic brace to engage
/// (resolution.md §1a: "prone on LOW+, kneeling on MID+, standing on HIGH"). A
/// faced cover band at or above the stance's gate grants the
/// [`BraceContribution`](crate::tuning::BraceContribution).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BraceMinHeight {
    /// Minimum faced cover band to brace while prone (doc: LOW+).
    pub prone: HeightBand,
    /// Minimum faced cover band to brace while kneeling (doc: MID+).
    pub kneel: HeightBand,
    /// Minimum faced cover band to brace while standing (doc: HIGH).
    pub stand: HeightBand,
}

impl Default for BraceMinHeight {
    fn default() -> Self {
        // Brace min-height gate from resolution.md §1a: prone↔LOW+, kneel↔MID+,
        // stand↔HIGH. These are band thresholds, not magnitudes.
        Self {
            prone: HeightBand::Low,
            kneel: HeightBand::Mid,
            stand: HeightBand::High,
        }
    }
}

/// One sampled point on a [`StabilityCurve`] — a `(score, output)` pair.
///
/// `score` is a point on the 0–100 stability axis; `output` is the curve's value
/// there (a cone multiplier for the cone-mult curve, a recoil-growth coefficient
/// for the recoil-growth curve). Both are [`StabilityCurveCoord`] — coordinates of
/// the same sampled point.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct StabilityCurvePoint {
    /// The stability score this point is sampled at (the curve's x, 0–100).
    pub score:  StabilityCurveCoord,
    /// The curve's output at that score (the curve's y).
    pub output: StabilityCurveCoord,
}

/// A **stability curve** — the ordered sample points mapping the 0–100 stability
/// score to a curve output (resolution.md §1a: "Normalised over 100 and fed
/// through a tuning curve"). Two such curves read off one score: the cone-mult
/// curve (steadier → narrower) and the recoil-growth curve (steadier → climbs
/// less).
///
/// A `Vec` of sample points (the curve is authored as a lookup, the equation
/// *form* — interpolation — lives in code in a later slice). Distinct from any
/// other list by its newtype.
#[derive(Debug, Clone, PartialEq, bevy::prelude::Deref, Deserialize)]
#[serde(transparent)]
pub struct StabilityCurve(Vec<StabilityCurvePoint>);

impl StabilityCurve {
    /// Build a stability curve from its ordered sample points.
    #[must_use]
    pub const fn new(points: Vec<StabilityCurvePoint>) -> Self {
        Self(points)
    }
}

/// The two **stability curves** read off the single 0–100 stability score
/// (resolution.md §1a: "two curve reads off one 0–100 score").
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StabilityCurves {
    /// The cone-mult curve — steadier scores yield a smaller multiplier (narrower
    /// cone).
    pub cone_mult:     StabilityCurve,
    /// The recoil-growth curve — steadier scores yield less recoil climb.
    pub recoil_growth: StabilityCurve,
}

impl Default for StabilityCurves {
    fn default() -> Self {
        // Placeholder two-point curves over the 0–100 score: a steadier score
        // (100) yields a narrower cone and less recoil growth than a shaky one
        // (0). TUNABLE shape — only the mechanism is fixed; tests are
        // value-agnostic.
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

/// The **aim-mode** coefficients — the cone multiplier and TU premium when aiming
/// rather than hip-firing (resolution.md §1a: aimed ×0.6 cone, ×1.5 TU; hip = 1).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct AimMode {
    /// The cone multiplier when aiming (doc default ×0.6; hip-fired = 1).
    pub cone_mult:  AimConeMult,
    /// The TU-cost multiplier when aiming (doc default ×1.5).
    pub tu_premium: AimTuPremium,
}

impl Default for AimMode {
    fn default() -> Self {
        // Aim-mode coefficients from resolution.md §1a (×0.6 cone, ×1.5 TU) —
        // tunable, not pinned.
        Self {
            cone_mult:  AimConeMult::new(0.6),
            tu_premium: AimTuPremium::new(1.5),
        }
    }
}

/// The **concentration-p coefficients** — the data-driven mapping from
/// `Shooting × weapon.accuracy` to the in-cone power-law exponent `p`
/// (resolution.md §1b). With these, `concentration_p(Shooting, weapon.accuracy)`
/// is a data edit, not a code change.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ConcentrationCoeffs {
    /// The base exponent at zero accuracy (`p ≈ 1` scatters evenly — resolution.md
    /// §1b).
    pub base:  ConcentrationCoeff,
    /// The per-(Shooting × accuracy) scale that raises `p` toward dead-center
    /// clustering.
    pub scale: ConcentrationCoeff,
}

impl Default for ConcentrationCoeffs {
    fn default() -> Self {
        // Placeholder coefficients for the resolution.md §1b `p` curve: base ≈ 1
        // (even scatter at low accuracy), a positive per-accuracy scale. TUNABLE.
        Self {
            base:  ConcentrationCoeff::new(1.0),
            scale: ConcentrationCoeff::new(1.0),
        }
    }
}

/// The per-stance **muzzle heights** — the de-pxed `shot_z_by_stance` level-fractions
/// (battle-space.md §"Stance / cover / muzzle / aim heights"). Universal tuning,
/// authored to land in design-sensible clearance bands.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MuzzleHeights {
    /// Prone muzzle height (fires low — can't clear even LOW cover; doc intent).
    pub prone: MuzzleHeight,
    /// Kneeling muzzle height (fires Mid).
    pub kneel: MuzzleHeight,
    /// Standing muzzle height (fires High).
    pub stand: MuzzleHeight,
}

impl Default for MuzzleHeights {
    fn default() -> Self {
        // Per-stance muzzle level-fractions authored to sit in the LOW / MID /
        // HIGH clearance bands (battle-space.md §"Banding" defaults ≈ ⅓, ⅔). The
        // exact fractions are TUNABLE — value-agnostic tests only.
        Self {
            prone: MuzzleHeight::new(0.15),
            kneel: MuzzleHeight::new(0.45),
            stand: MuzzleHeight::new(0.8),
        }
    }
}

/// The per-stance **silhouette tops** — the tunable level-fractions of a ganger's
/// silhouette top per stance, the aim-point source (battle-space.md §"Stance /
/// cover / muzzle / aim heights"). Universal tuning, **moved here from ganger data**
/// by this ticket — there is no per-ganger size model.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SilhouetteTops {
    /// Prone silhouette top (the lowest profile).
    pub prone: SilhouetteTop,
    /// Kneeling silhouette top (compressed).
    pub kneel: SilhouetteTop,
    /// Standing silhouette top (full height).
    pub stand: SilhouetteTop,
}

impl Default for SilhouetteTops {
    fn default() -> Self {
        // Per-stance silhouette-top level-fractions (prone lowest, standing
        // tallest). TUNABLE — value-agnostic tests only.
        Self {
            prone: SilhouetteTop::new(0.3),
            kneel: SilhouetteTop::new(0.6),
            stand: SilhouetteTop::new(0.95),
        }
    }
}

/// The §1 **cone / stability / recoil / aim** tuning extension — every coefficient
/// the resolution.md §1 cone/stability/recoil/aim math reads (the data substrate
/// for the rest of E2).
///
/// Bundles the stance + brace stability contributions and the brace min-height
/// gate, the two stability curves, the aim-mode coefficients, the recoil-climb
/// coefficient, the concentration-p coefficients, and the de-pxed muzzle/aim
/// geometry (the aim-height fraction, the cell-fraction forward offset, and the
/// per-stance muzzle + silhouette-top level-fractions). All **tuning
/// COEFFICIENTS** — weapon numbers live as the per-stat [`crate::weapon`] components
/// on the armed entity, not here.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ConeStabilityTuning {
    /// Per-stance stability contributions (prone 40 / kneel 25 / stand 10).
    pub stance_stability:            StanceStability,
    /// The auto-brace contribution (+30) added when the faced cover suits the stance.
    pub brace_contribution:          BraceContribution,
    /// The GTW-542 sight-attachment stability bonus — points a [`Scoped`](crate::weapon::Scoped)
    /// weapon adds to the score (a scoped weapon aims steadier → a tighter cone).
    pub sight_stability_bonus:       SightStabilityBonus,
    /// The GTW-543 emplacement stability bonus — points a ganger MANNING a weapon emplacement
    /// (firing the bolted-down [`MountedWeapon`](crate::weapon::MountedWeapon)) adds to the score
    /// (a fixed mount aims steadier → a tighter cone, offsetting the mounted gun's low accuracy).
    pub emplacement_stability_bonus: EmplacementStabilityBonus,
    /// The per-stance brace min-height gate (prone↔LOW+, kneel↔MID+, stand↔HIGH).
    pub brace_min_height:            BraceMinHeight,
    /// The two stability curves (cone-mult + recoil-growth) over the 0–100 score.
    pub stability_curves:            StabilityCurves,
    /// The aim-mode cone multiplier (×0.6) and TU premium (×1.5).
    pub aim_mode:                    AimMode,
    /// The recoil-climb coefficient (the per-prior-shot upward axis tilt).
    pub recoil_climb:                RecoilClimb,
    /// The concentration-p coefficients (so `concentration_p` is data-driven).
    pub concentration:               ConcentrationCoeffs,
    /// The aim-height fraction of the target's silhouette top (a level-fraction).
    pub aim_height_frac:             AimHeightFrac,
    /// The muzzle forward offset along the facing (a cell-fraction).
    pub muzzle_forward_offset:       MuzzleForwardOffset,
    /// The per-stance muzzle height level-fractions (de-pxed `shot_z_by_stance`).
    pub muzzle_heights:              MuzzleHeights,
    /// The per-stance silhouette-top level-fractions (the aim-point source).
    pub silhouette_tops:             SilhouetteTops,
}

impl Default for ConeStabilityTuning {
    fn default() -> Self {
        // The auto-brace contribution (+30) and recoil-climb coefficient from
        // resolution.md §1a — tunable, value-agnostic tests only. The sub-structs
        // carry their own doc-default impls.
        Self {
            stance_stability:            StanceStability::default(),
            brace_contribution:          BraceContribution::new(30.0),
            // A modest steadying (+15, half the brace) — a defensible-but-arbitrary
            // starting point; tests assert only the STEADIER / NARROWER-cone invariant,
            // never this magnitude (the SuppressionStabilityPenalty precedent).
            sight_stability_bonus:       SightStabilityBonus::new(15.0),
            // A substantial steadying (+40 — a fixed mount is the steadiest firing position,
            // topping the +30 brace) offsetting the mounted gun's low base accuracy. A
            // defensible-but-arbitrary starting point; tests assert only the STEADIER /
            // NARROWER-cone invariant, never this magnitude.
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
