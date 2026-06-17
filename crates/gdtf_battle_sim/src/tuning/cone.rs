//! The §1 cone / stability / recoil / aim **leaf coefficient newtypes**
//! (resolution.md §1a / §1b; battle-space.md §"Stance / cover / muzzle / aim
//! heights"). The grouping structs that compose these live in
//! [`cone_groups`](crate::tuning::cone_groups).

use bevy::prelude::Deref;
use serde::Deserialize;

/// A **stance stability contribution** — the points a stance adds to the 0–100
/// stability score (resolution.md §1a: "prone 40 / kneel 25 / stand 10"). Steadier
/// stances contribute more, narrowing the cone via the stability curve.
///
/// A tuning COEFFICIENT (one newtype shared by the three per-stance fields of
/// [`StanceStability`](crate::tuning::StanceStability): each is the same *kind* of
/// value, a stance's stability points). Private inner + derived [`Deref`];
/// `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct StanceContribution(pub(super) f32);

/// The **auto-brace contribution** — the points automatic bracing adds to the
/// stability score (resolution.md §1a: "+30 when the faced cell's cover height
/// suits the stance"). Stacks on the stance contribution before the curve read.
///
/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BraceContribution(pub(super) f32);

/// A point sampled on a **stability curve** — one `(score, output)` pair (the 0–100
/// stability score on the x axis, the curve's multiplier/coefficient on the y
/// axis). Two curves read off the same score (resolution.md §1a): the cone-mult
/// curve (steadier → narrower) and the recoil-growth curve (steadier → climbs
/// less).
///
/// A tuning COEFFICIENT (one newtype reused by both axes of a
/// [`StabilityCurvePoint`](crate::tuning::StabilityCurvePoint): the score input and
/// the curve output are both `f32` coordinates of one sampled point). Private inner +
/// derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct StabilityCurveCoord(pub(super) f32);

/// The **aim-mode cone multiplier** — the `aim` term of `θ_cone` when aiming
/// (resolution.md §1a: aimed narrows ×0.6; hip-fired = 1). A multiplier on the
/// cone's angular size.
///
/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct AimConeMult(pub(super) f32);

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
pub struct AimTuPremium(pub(super) f32);

/// The **recoil-climb coefficient** — the per-prior-shot upward axis tilt
/// (resolution.md §1a: round *i*'s axis tilts up by `prior_shots × recoil_climb ×
/// recoil_growth` radians). Scales how fast the muzzle walks up during a burst,
/// before stability's recoil-growth curve damps it.
///
/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct RecoilClimb(pub(super) f32);

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
/// two fields of [`ConcentrationCoeffs`](crate::tuning::ConcentrationCoeffs) (a base
/// and a per-accuracy scale): both are the same *kind* of value, a coefficient of
/// the `p` curve.
///
/// A tuning COEFFICIENT. Private inner + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ConcentrationCoeff(pub(super) f32);

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
pub struct AimHeightFrac(pub(super) f32);

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
pub struct MuzzleForwardOffset(pub(super) f32);

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
/// [`MuzzleHeights`](crate::tuning::MuzzleHeights): each is the same *kind* of value,
/// a per-stance muzzle level-fraction). Private inner + derived [`Deref`];
/// `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MuzzleHeight(pub(super) f32);

/// A **per-stance silhouette-top height** — the tunable **level-fraction** of a
/// ganger's silhouette top for one stance (the aim-point source: aim z = this ×
/// [`AimHeightFrac`]; battle-space.md §"Stance / cover / muzzle / aim heights").
/// **Universal tuning, not per-ganger data** (there is no per-ganger size model) —
/// this ticket moves the silhouette-top home from ganger data to tuning.
///
/// A tuning COEFFICIENT (one newtype shared by the three per-stance fields of
/// [`SilhouetteTops`](crate::tuning::SilhouetteTops): each is the same *kind* of
/// value, a per-stance silhouette-top level-fraction). Private inner + derived
/// [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct SilhouetteTop(pub(super) f32);

impl SilhouetteTop {
    /// Build a silhouette-top from its magnitude (a dimensionless level-fraction of a
    /// ganger's silhouette-top height for one stance; TBD tuning).
    #[must_use]
    pub const fn new(top: f32) -> Self {
        Self(top)
    }
}
