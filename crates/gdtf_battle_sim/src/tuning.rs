//! Combat tuning data — the single home for every balance coefficient.
//!
//! The equation *forms* live in code (`docs/combat/resolution.md`); every
//! *coefficient* is a default here, so balancing is a data edit, not a code
//! change ("Coefficients live in the combat-tuning data", resolution.md §"What's
//! pure math vs sim"). [`CombatTuning`] is a Bevy [`Resource`] that
//! deserializes from a `.ron` file, and **no numeric tuning literal lives
//! anywhere outside this module** — the [`crate::metric::MAX_LEVELS`]
//! coordinate-system constant is the only other named number.
//!
//! Every numeric leaf is a named newtype (no bare `f32`/`u16` field), per the
//! no-bare-types rule: each carries a derived [`Deref`] to its inner value and
//! `#[serde(transparent)]` so it round-trips as a bare RON scalar.

use bevy::prelude::{Deref, Resource};
use serde::Deserialize;

use crate::{cover::HeightBand, matchup::MatchupMultiplier};

/// A projectile clearance band edge, as a **level-fraction** — a dimensionless
/// fraction of one level's height (`z ∈ [0,1)` within a storey).
///
/// One newtype shared by **both** band edges of [`ProjectileBandEdges`]: the two
/// edges are the same *kind* of value (a level-fraction clearance threshold),
/// distinguished by their field. Because they are fractions of a storey they
/// re-scale with the cubic voxel and carry no pixel (`docs/combat/battle-space.md`
/// §"Banding"). `#[serde(transparent)]` lets it parse a bare RON scalar.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BandEdge(f32);

impl BandEdge {
    /// Build a band-edge level-fraction from its magnitude (a fraction of one
    /// level's height) — for tests and programmatic tuning edits; shipped values
    /// come from the `.ron` via the derived [`Deserialize`].
    #[must_use]
    pub const fn new(edge: f32) -> Self {
        Self(edge)
    }
}

/// `j` — the penetrating-damage scale: how hard pen damage pushes severity up
/// (resolution.md §6 severity score).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct PenDamageScale(f32);

/// `k` — Toughness mitigation: how much the defender's Toughness subtracts from
/// the severity score (resolution.md §6).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ToughnessMitigation(f32);

/// `I` — shooter-luck scale: the shooter's Luck adds to the severity score,
/// nudging toward nastier wounds (resolution.md §6).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ShooterLuckScale(f32);

/// `L` — defender-luck spread cap: the defender's Luck shrinks the random spread
/// (`R − L × Luck_defender`), capping the bad tail (resolution.md §6).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct DefenderLuckSpreadCap(f32);

/// `R` — the one-sided random spread before the defender's Luck trims it
/// (resolution.md §6: `roll(0..R_eff)`).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct RandomSpread(f32);

/// `R_min` — the floor on the trimmed spread (`max(R_min, …)`), so the
/// defender's Luck can never erase the random term entirely (resolution.md §6).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct RandomSpreadMin(f32);

/// The relative weight of one body part in the §4 `roll_body_part` weighted roll.
///
/// One newtype reused by all six fields of [`BodyPartWeights`]: each part's
/// weight is the same *kind* of value (a relative pick weight), distinguished by
/// its field. A small non-negative integer summed into a weighted pick;
/// `#[serde(transparent)]` lets it parse a bare RON scalar.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct BodyPartWeight(u16);

impl BodyPartWeight {
    /// Build a body-part pick weight from its relative magnitude (TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u16` private (house
    /// style) while letting callers (e.g. the `roll_body_part` roll's tests, or
    /// any code assembling a [`BodyPartWeights`] outside this module) build a
    /// weight without a bare `u16` escaping.
    #[must_use]
    pub const fn new(weight: u16) -> Self {
        Self(weight)
    }
}

/// The projectile clearance band edges, as **level-fractions** within one
/// level's height.
///
/// The march bands each crossed cell LOW / MID / HIGH by the round's continuous
/// `z` within the crossed level and compares it to the occupant's band
/// (resolution.md §2). The edges are dimensionless fractions of a storey
/// (defaults ≈ ⅓ and ⅔ of a level), so they re-scale with the cubic voxel and
/// carry no pixel (`docs/combat/battle-space.md` §"Banding"). `low_mid` is the
/// LOW→MID edge, `mid_high` the MID→HIGH edge.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ProjectileBandEdges {
    /// The LOW→MID clearance edge (level-fraction). Default ≈ ⅓ of a level.
    pub low_mid:  BandEdge,
    /// The MID→HIGH clearance edge (level-fraction). Default ≈ ⅔ of a level.
    pub mid_high: BandEdge,
}

impl Default for ProjectileBandEdges {
    fn default() -> Self {
        // Tunable level-fraction defaults (≈ ⅓ and ⅔ of a level) from
        // docs/combat/battle-space.md §"Banding". These are balance data, not a
        // coordinate-system fact — value-agnostic tests only.
        Self {
            low_mid:  BandEdge(0.33),
            mid_high: BandEdge(0.67),
        }
    }
}

/// The wound-severity scaling scalars from resolution.md §6.
///
/// They scale the severity score (form in resolution.md §6): `pen_damage_scale`
/// times penetrating damage, minus `toughness_mitigation` times Toughness, plus
/// the part modifier, the weapon fatal-bias, and `shooter_luck_scale` times the
/// shooter's Luck, plus a one-sided random draw over `0..R_eff` where
/// `R_eff = max(random_spread_min, random_spread - defender_luck_spread_cap *
/// Luck_defender)`. Each field is its own named newtype over the doc symbol
/// (`j`/`k`/`I`/`L`/`R`/`R_min`) so distinct concepts stay distinct types;
/// magnitudes are tunable defaults.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct SeverityScaling {
    /// `j` — penetrating-damage scale: how hard pen damage pushes severity up.
    pub pen_damage_scale:         PenDamageScale,
    /// `k` — Toughness mitigation: how much the defender's Toughness subtracts.
    pub toughness_mitigation:     ToughnessMitigation,
    /// `I` — shooter-luck scale: the shooter's Luck adds (nastier wounds).
    pub shooter_luck_scale:       ShooterLuckScale,
    /// `L` — defender-luck spread cap: the defender's Luck shrinks the random
    /// spread (`R − L × Luck_defender`).
    pub defender_luck_spread_cap: DefenderLuckSpreadCap,
    /// `R` — the one-sided random spread before the defender's Luck trims it.
    pub random_spread:            RandomSpread,
    /// `R_min` — the floor on the trimmed spread (`max(R_min, …)`), so Luck can
    /// never erase the random term entirely.
    pub random_spread_min:        RandomSpreadMin,
}

impl Default for SeverityScaling {
    fn default() -> Self {
        // TUNABLE placeholder magnitudes for the resolution.md §6 symbols — the
        // forms are fixed, these numbers are balance data (de-brittled: tests
        // exercise the serde mechanism, not these shipped values).
        Self {
            pen_damage_scale:         PenDamageScale(1.0),
            toughness_mitigation:     ToughnessMitigation(1.0),
            shooter_luck_scale:       ShooterLuckScale(1.0),
            defender_luck_spread_cap: DefenderLuckSpreadCap(1.0),
            random_spread:            RandomSpread(10.0),
            random_spread_min:        RandomSpreadMin(1.0),
        }
    }
}

/// The body-part hit-location weights — the relative weight of each of the six
/// parts in the §4 `roll_body_part` weighted roll.
///
/// Head is rare, torso the bulk (resolution.md §4: Head 6 / Torso 40 / each Arm
/// 12 / each Leg 15). Each field is a [`BodyPartWeight`]; the magnitudes are
/// tunable balance data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BodyPartWeights {
    /// Head weight — rare (doc default 6).
    pub head:      BodyPartWeight,
    /// Torso weight — the bulk of hits (doc default 40).
    pub torso:     BodyPartWeight,
    /// Left-arm weight (doc default 12).
    pub left_arm:  BodyPartWeight,
    /// Right-arm weight (doc default 12).
    pub right_arm: BodyPartWeight,
    /// Left-leg weight (doc default 15).
    pub left_leg:  BodyPartWeight,
    /// Right-leg weight (doc default 15).
    pub right_leg: BodyPartWeight,
}

impl Default for BodyPartWeights {
    fn default() -> Self {
        // Defaults from docs/combat/resolution.md §4.
        Self {
            head:      BodyPartWeight(6),
            torso:     BodyPartWeight(40),
            left_arm:  BodyPartWeight(12),
            right_arm: BodyPartWeight(12),
            left_leg:  BodyPartWeight(15),
            right_leg: BodyPartWeight(15),
        }
    }
}

/// A **stance stability contribution** — the points a stance adds to the 0–100
/// stability score (resolution.md §1a: "prone 40 / kneel 25 / stand 10"). Steadier
/// stances contribute more, narrowing the cone via the stability curve.
///
/// A tuning COEFFICIENT (one newtype shared by the three per-stance fields of
/// [`StanceStability`]: each is the same *kind* of value, a stance's stability
/// points). Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct StanceContribution(f32);

/// The **auto-brace contribution** — the points automatic bracing adds to the
/// stability score (resolution.md §1a: "+30 when the faced cell's cover height
/// suits the stance"). Stacks on the stance contribution before the curve read.
///
/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BraceContribution(f32);

/// A point sampled on a **stability curve** — one `(score, output)` pair (the 0–100
/// stability score on the x axis, the curve's multiplier/coefficient on the y
/// axis). Two curves read off the same score (resolution.md §1a): the cone-mult
/// curve (steadier → narrower) and the recoil-growth curve (steadier → climbs
/// less).
///
/// A tuning COEFFICIENT (one newtype reused by both axes of a [`StabilityCurvePoint`]:
/// the score input and the curve output are both `f32` coordinates of one sampled
/// point). Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct StabilityCurveCoord(f32);

/// The **aim-mode cone multiplier** — the `aim` term of `θ_cone` when aiming
/// (resolution.md §1a: aimed narrows ×0.6; hip-fired = 1). A multiplier on the
/// cone's angular size.
///
/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct AimConeMult(f32);

impl AimConeMult {
    /// Build an aim-mode cone multiplier from its magnitude (a dimensionless
    /// angular scale).
    #[must_use]
    pub const fn new(mult: f32) -> Self {
        Self(mult)
    }

    /// The **hip-fired** aim multiplier — the identity `1.0` (resolution.md §1a:
    /// "hip-fired = 1"). The `aim` term when the shooter is not aiming, so it
    /// leaves `θ_cone` unchanged. Not a tunable magnitude — the multiplicative
    /// identity, so a hip-fired shot is exactly the un-narrowed cone.
    #[must_use]
    pub const fn hip_fired() -> Self {
        Self(1.0)
    }
}

/// The **aim-mode TU premium** — the multiplier on a shot's TU cost when aiming
/// (resolution.md §1a: "the tradeoff is TU (×1.5 shot cost)"). The cost lever
/// against the ×0.6 cone narrowing.
///
/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct AimTuPremium(f32);

/// The **recoil-climb coefficient** — the per-prior-shot upward axis tilt
/// (resolution.md §1a: round *i*'s axis tilts up by `prior_shots × recoil_climb ×
/// recoil_growth` radians). Scales how fast the muzzle walks up during a burst,
/// before stability's recoil-growth curve damps it.
///
/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct RecoilClimb(f32);

impl RecoilClimb {
    /// Build a recoil-climb coefficient from its magnitude (the per-prior-shot
    /// upward axis tilt in radians; TBD tuning).
    #[must_use]
    pub const fn new(climb: f32) -> Self {
        Self(climb)
    }
}

/// A **concentration-p coefficient** — a scalar of the data-driven
/// `p = concentration_p(Shooting, weapon.accuracy)` mapping (resolution.md §1b:
/// the in-cone power-law exponent rising with accuracy). One newtype shared by the
/// two fields of [`ConcentrationCoeffs`] (a base and a per-accuracy scale): both
/// are the same *kind* of value, a coefficient of the `p` curve.
///
/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ConcentrationCoeff(f32);

impl ConcentrationCoeff {
    /// Build a concentration-p coefficient from its magnitude (a scalar of the
    /// data-driven `p` curve — a `base` exponent or a per-accuracy `scale`; TBD
    /// tuning).
    #[must_use]
    pub const fn new(coeff: f32) -> Self {
        Self(coeff)
    }
}

/// The **aim-height fraction** — the dimensionless level-fraction of the target's
/// silhouette-top height used as the aim-point z (resolution.md §1; battle-space.md
/// §"Stance / cover / muzzle / aim heights": "the target's silhouette-top
/// level-fraction × `aim_height_frac`"). No pixel magnitude — a fraction of the
/// target's own band-top.
///
/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct AimHeightFrac(f32);

impl AimHeightFrac {
    /// Build an aim-height fraction from its magnitude (a dimensionless
    /// level-fraction of the target's silhouette-top height; TBD tuning).
    #[must_use]
    pub const fn new(frac: f32) -> Self {
        Self(frac)
    }
}

/// The **muzzle forward offset** — the de-pxed barrel offset, expressed as a
/// **cell-fraction** along the shooter's facing (battle-space.md §"Sub-cell
/// precision on the ground plane": "a fraction of a cell along the facing's
/// forward vector; clamped so it can never leave the cell" — never a pixel). The
/// muzzle is `cell_center + this × facing` on the ground plane.
///
/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MuzzleForwardOffset(f32);

impl MuzzleForwardOffset {
    /// Build a muzzle forward offset from its magnitude (a cell-fraction along the
    /// shooter's facing; TBD tuning).
    #[must_use]
    pub const fn new(offset: f32) -> Self {
        Self(offset)
    }
}

/// A **per-stance muzzle height** — the de-pxed `shot_z_by_stance`, a tunable
/// **level-fraction** of the shot's launch z for one stance (battle-space.md
/// §"Stance / cover / muzzle / aim heights": "Muzzle height by stance → a tunable
/// level-fraction (prone / kneel / stand each author their own)"). Authored to land
/// in design-sensible clearance bands (prone fires low, kneeling Mid, standing
/// High). **Universal tuning, not per-ganger data** (there is no per-ganger size
/// model).
///
/// A tuning COEFFICIENT (one newtype shared by the three per-stance fields of
/// [`MuzzleHeights`]: each is the same *kind* of value, a per-stance muzzle
/// level-fraction). Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MuzzleHeight(f32);

/// A **per-stance silhouette-top height** — the tunable **level-fraction** of a
/// ganger's silhouette top for one stance (the aim-point source: aim z = this ×
/// [`AimHeightFrac`]; battle-space.md §"Stance / cover / muzzle / aim heights").
/// **Universal tuning, not per-ganger data** (there is no per-ganger size model) —
/// this ticket moves the silhouette-top home from ganger data to tuning.
///
/// A tuning COEFFICIENT (one newtype shared by the three per-stance fields of
/// [`SilhouetteTops`]: each is the same *kind* of value, a per-stance silhouette-top
/// level-fraction). Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct SilhouetteTop(f32);

impl SilhouetteTop {
    /// Build a silhouette-top from its magnitude (a dimensionless level-fraction of a
    /// ganger's silhouette-top height for one stance; TBD tuning).
    #[must_use]
    pub const fn new(top: f32) -> Self {
        Self(top)
    }
}

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
            prone: StanceContribution(40.0),
            kneel: StanceContribution(25.0),
            stand: StanceContribution(10.0),
        }
    }
}

/// The per-stance **brace min-height gate** — the minimum cover [`HeightBand`] a
/// faced cell must reach for each stance's automatic brace to engage
/// (resolution.md §1a: "prone on LOW+, kneeling on MID+, standing on HIGH"). A
/// faced cover band at or above the stance's gate grants the [`BraceContribution`].
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
#[derive(Debug, Clone, PartialEq, Deref, Deserialize)]
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
                    score:  StabilityCurveCoord(0.0),
                    output: StabilityCurveCoord(1.0),
                },
                StabilityCurvePoint {
                    score:  StabilityCurveCoord(100.0),
                    output: StabilityCurveCoord(0.5),
                },
            ]),
            recoil_growth: StabilityCurve::new(vec![
                StabilityCurvePoint {
                    score:  StabilityCurveCoord(0.0),
                    output: StabilityCurveCoord(1.0),
                },
                StabilityCurvePoint {
                    score:  StabilityCurveCoord(100.0),
                    output: StabilityCurveCoord(0.25),
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
            cone_mult:  AimConeMult(0.6),
            tu_premium: AimTuPremium(1.5),
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
            base:  ConcentrationCoeff(1.0),
            scale: ConcentrationCoeff(1.0),
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
            prone: MuzzleHeight(0.15),
            kneel: MuzzleHeight(0.45),
            stand: MuzzleHeight(0.8),
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
            prone: SilhouetteTop(0.3),
            kneel: SilhouetteTop(0.6),
            stand: SilhouetteTop(0.95),
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
/// COEFFICIENTS** — weapon numbers live on [`crate::weapon::Weapon`].
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ConeStabilityTuning {
    /// Per-stance stability contributions (prone 40 / kneel 25 / stand 10).
    pub stance_stability:      StanceStability,
    /// The auto-brace contribution (+30) added when the faced cover suits the stance.
    pub brace_contribution:    BraceContribution,
    /// The per-stance brace min-height gate (prone↔LOW+, kneel↔MID+, stand↔HIGH).
    pub brace_min_height:      BraceMinHeight,
    /// The two stability curves (cone-mult + recoil-growth) over the 0–100 score.
    pub stability_curves:      StabilityCurves,
    /// The aim-mode cone multiplier (×0.6) and TU premium (×1.5).
    pub aim_mode:              AimMode,
    /// The recoil-climb coefficient (the per-prior-shot upward axis tilt).
    pub recoil_climb:          RecoilClimb,
    /// The concentration-p coefficients (so `concentration_p` is data-driven).
    pub concentration:         ConcentrationCoeffs,
    /// The aim-height fraction of the target's silhouette top (a level-fraction).
    pub aim_height_frac:       AimHeightFrac,
    /// The muzzle forward offset along the facing (a cell-fraction).
    pub muzzle_forward_offset: MuzzleForwardOffset,
    /// The per-stance muzzle height level-fractions (de-pxed `shot_z_by_stance`).
    pub muzzle_heights:        MuzzleHeights,
    /// The per-stance silhouette-top level-fractions (the aim-point source).
    pub silhouette_tops:       SilhouetteTops,
}

impl Default for ConeStabilityTuning {
    fn default() -> Self {
        // The auto-brace contribution (+30) and recoil-climb coefficient from
        // resolution.md §1a — tunable, value-agnostic tests only. The sub-structs
        // carry their own doc-default impls.
        Self {
            stance_stability:      StanceStability::default(),
            brace_contribution:    BraceContribution(30.0),
            brace_min_height:      BraceMinHeight::default(),
            stability_curves:      StabilityCurves::default(),
            aim_mode:              AimMode::default(),
            recoil_climb:          RecoilClimb(0.01),
            concentration:         ConcentrationCoeffs::default(),
            aim_height_frac:       AimHeightFrac(1.0),
            muzzle_forward_offset: MuzzleForwardOffset(0.3),
            muzzle_heights:        MuzzleHeights::default(),
            silhouette_tops:       SilhouetteTops::default(),
        }
    }
}

/// The 7-type **matchup multipliers** — the punch-&-shred scalars each
/// [`crate::matchup::Matchup`] outcome applies (`docs/combat/matchup.md`
/// §"Modifier never auto-win": favorable ×1.33 / neutral ×1.0 / resisted ×0.34).
///
/// Three [`MatchupMultiplier`] tuning COEFFICIENTS, one per outcome. The
/// asymmetry is deliberate: the resisted penalty (−66%) is **double** the
/// favorable bonus (+33%), pressuring players to *avoid* bad matchups, not just
/// chase good ones. Scale-independent — a proportional swing is always felt and
/// never auto-wins (matchup.md §"Why a multiplier"). The matchup modifies punch &
/// shred ONLY (resolution.md §5); these scalars never touch any other stat.
/// Magnitudes are **tunable** — value-agnostic tests only (ordering, not value).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MatchupMultipliers {
    /// Favorable — the weapon type beats the armor type (doc default ×1.33).
    pub favorable: MatchupMultiplier,
    /// Neutral — same wheel node / mirror (doc default ×1.0, unchanged).
    pub neutral:   MatchupMultiplier,
    /// Resisted — the weapon type loses to the armor type (doc default ×0.34).
    pub resisted:  MatchupMultiplier,
}

impl Default for MatchupMultipliers {
    fn default() -> Self {
        // Matchup multipliers from docs/combat/matchup.md §"Modifier never
        // auto-win" (favorable ×1.33 / neutral ×1.0 / resisted ×0.34). The
        // resisted penalty is double the favorable bonus by design — tunable
        // balance data, asserted by ordering, never by magnitude.
        Self {
            favorable: MatchupMultiplier::new(1.33),
            neutral:   MatchupMultiplier::new(1.0),
            resisted:  MatchupMultiplier::new(0.34),
        }
    }
}

/// The combat tuning resource — every balance coefficient the sim marches with.
///
/// A Bevy [`Resource`] deserializable from a `.ron` file (the tuning store is a
/// serde-loaded resource, resolution.md §"Coefficients live in the
/// combat-tuning data"). Holds the clearance band edges, the §6 severity
/// scaling, the body-part weights, and the E2.1 §1 cone/stability/recoil/aim
/// extension ([`ConeStabilityTuning`]); more sub-fields land as the systems do.
/// Defaults carry the doc values, but they are **tunable** — a data file
/// overrides any of them.
#[derive(Debug, Clone, PartialEq, Default, Resource, Deserialize)]
pub struct CombatTuning {
    /// Projectile clearance band edges (the LOW/MID/HIGH thresholds).
    pub projectile_band_edges: ProjectileBandEdges,
    /// The resolution.md §6 wound-severity scaling scalars.
    pub severity_scaling:      SeverityScaling,
    /// The §4 body-part hit-location weights.
    pub body_part_weights:     BodyPartWeights,
    /// The §1 cone / stability / recoil / aim coefficients (E2.1) — the data
    /// substrate for the rest of E2.
    pub cone_stability:        ConeStabilityTuning,
    /// The 7-type matchup multipliers (E3.2) — the favorable / neutral / resisted
    /// punch-&-shred scalars.
    pub matchup_multipliers:   MatchupMultipliers,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each tuning newtype wraps the right inner type and its derived [`Deref`]
    /// reaches that inner value (C9/C10/C11 mandate a derived `Deref` on every
    /// tuning newtype; this exercises that surface so dropping the derive would
    /// fail a test).
    ///
    /// Built from **arbitrary** literals, never the shipped/default magnitudes:
    /// this pins the Deref *target type and mechanism*, not a balance value, so
    /// it stays non-brittle against a tuning edit. The seven `f32` newtypes are
    /// compared by bit pattern (the literals are exactly representable, so this
    /// is an exact integer equality — no `float_cmp` lint, no epsilon).
    #[test]
    fn tuning_newtypes_wrap_inner_and_deref() {
        // Each f32 newtype: deref reaches the inner f32 (bit-exact arbitrary
        // value, not the default).
        assert_eq!((*BandEdge(5.0)).to_bits(), 5.0_f32.to_bits());
        assert_eq!((*PenDamageScale(2.5)).to_bits(), 2.5_f32.to_bits());
        assert_eq!((*ToughnessMitigation(3.5)).to_bits(), 3.5_f32.to_bits());
        assert_eq!((*ShooterLuckScale(4.5)).to_bits(), 4.5_f32.to_bits());
        assert_eq!((*DefenderLuckSpreadCap(6.5)).to_bits(), 6.5_f32.to_bits());
        assert_eq!((*RandomSpread(7.5)).to_bits(), 7.5_f32.to_bits());
        assert_eq!((*RandomSpreadMin(8.5)).to_bits(), 8.5_f32.to_bits());
        // The u16 newtype: deref reaches the inner u16 (arbitrary value).
        assert_eq!(*BodyPartWeight(3), 3u16);
    }

    /// C3 — every E2.1 cone/stability/recoil/aim extension leaf wraps the right
    /// inner type and its derived [`Deref`] reaches it. Built from **arbitrary**
    /// literals (never the shipped/default magnitudes), so this pins the Deref
    /// mechanism + target type, not a balance value (bit-exact f32 equality on
    /// exactly-representable literals — no `float_cmp` lint).
    #[test]
    fn cone_stability_newtypes_wrap_inner_and_deref() {
        assert_eq!((*StanceContribution(11.0)).to_bits(), 11.0_f32.to_bits());
        assert_eq!((*BraceContribution(22.0)).to_bits(), 22.0_f32.to_bits());
        assert_eq!((*StabilityCurveCoord(33.0)).to_bits(), 33.0_f32.to_bits());
        assert_eq!((*AimConeMult(0.25)).to_bits(), 0.25_f32.to_bits());
        assert_eq!((*AimTuPremium(1.25)).to_bits(), 1.25_f32.to_bits());
        assert_eq!((*RecoilClimb(0.5)).to_bits(), 0.5_f32.to_bits());
        assert_eq!((*ConcentrationCoeff(2.5)).to_bits(), 2.5_f32.to_bits());
        assert_eq!((*AimHeightFrac(0.75)).to_bits(), 0.75_f32.to_bits());
        assert_eq!((*MuzzleForwardOffset(0.125)).to_bits(), 0.125_f32.to_bits());
        assert_eq!((*MuzzleHeight(0.625)).to_bits(), 0.625_f32.to_bits());
        assert_eq!((*SilhouetteTop(0.875)).to_bits(), 0.875_f32.to_bits());

        // The curve newtype derefs to its inner Vec (arbitrary one-point shape).
        let curve = StabilityCurve::new(vec![StabilityCurvePoint {
            score:  StabilityCurveCoord(50.0),
            output: StabilityCurveCoord(0.7),
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

    /// C5/C3 — the E2.1 extension parses from a hand-written RON fragment with
    /// every numeric leaf a **bare scalar** (`#[serde(transparent)]`) and the
    /// stability curves authored as bare point lists. Value-agnostic: asserts only
    /// structural success (arbitrary literals, never shipped magnitudes).
    #[test]
    fn cone_stability_parses_from_ron_with_bare_scalar_leaves() {
        let ron = r"(
            stance_stability: ( prone: 40.0, kneel: 25.0, stand: 10.0 ),
            brace_contribution: 30.0,
            brace_min_height: ( prone: Low, kneel: Mid, stand: High ),
            stability_curves: (
                cone_mult:     [ ( score: 0.0, output: 1.0 ), ( score: 100.0, output: 0.5 ) ],
                recoil_growth: [ ( score: 0.0, output: 1.0 ), ( score: 100.0, output: 0.25 ) ],
            ),
            aim_mode: ( cone_mult: 0.6, tu_premium: 1.5 ),
            recoil_climb: 0.01,
            concentration: ( base: 1.0, scale: 1.0 ),
            aim_height_frac: 1.0,
            muzzle_forward_offset: 0.3,
            muzzle_heights: ( prone: 0.15, kneel: 0.45, stand: 0.8 ),
            silhouette_tops: ( prone: 0.3, kneel: 0.6, stand: 0.95 ),
        )";
        let parsed = ron::from_str::<ConeStabilityTuning>(ron);
        assert!(
            parsed.is_ok(),
            "the cone/stability tuning extension must deserialize from bare-scalar RON: {parsed:?}",
        );
    }

    /// The shipped `assets/combat/tuning.ron` deserializes into a
    /// [`CombatTuning`] on the **real** path (the same file the data-driven
    /// tuning store loads).
    ///
    /// Deliberately value-agnostic: the tuning magnitudes are the **tunable**
    /// balance data, so this pins only that the shipped file parses into the
    /// type — never a specific band edge, severity scalar, or part weight
    /// (asserting a magnitude would be brittle against a balance edit). The
    /// metric-const pin lives in [`crate::metric`] (a coordinate-system
    /// definition, not a tunable).
    #[test]
    fn shipped_tuning_ron_deserializes() {
        const SHIPPED_TUNING_RON: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/combat/tuning.ron"
        ));

        let parsed = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON);
        assert!(
            parsed.is_ok(),
            "shipped assets/combat/tuning.ron must deserialize into CombatTuning: {parsed:?}",
        );

        // AC4 — the E3.2 matchup-multiplier leaves are PRESENT in the shipped file
        // (the three fields parsed into the struct). Value-agnostic: it asserts the
        // ordering relation the leaves must hold (resisted < neutral < favorable),
        // never a magnitude — the multipliers are tunable balance data.
        let Ok(tuning) = parsed else {
            return;
        };
        let multipliers = tuning.matchup_multipliers;
        assert!(
            *multipliers.resisted < *multipliers.neutral,
            "shipped resisted multiplier must be < neutral",
        );
        assert!(
            *multipliers.neutral < *multipliers.favorable,
            "shipped neutral multiplier must be < favorable",
        );
    }
}
