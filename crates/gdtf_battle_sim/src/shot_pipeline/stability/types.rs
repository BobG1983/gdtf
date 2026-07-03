//! The §1a stability layer's **named values** — the [`StabilityTerms`] bundle of
//! per-shot zero-identity terms, the emplacement-contribution input, the normalised
//! [`StabilityScore`], the two distinct curve outputs ([`ConeMult`] /
//! [`RecoilGrowth`]), and the axis-agnostic [`CurveOutput`] the curve read returns
//! before it is given an axis meaning.

use bevy::prelude::Deref;

use crate::{
    stability::terrain_brace::TerrainBraced,
    weapon::{Stable, WeaponBraceBonus},
};

/// The **per-shot stability terms** — the four zero-identity inputs a caller
/// resolves before the §1a score is read (GTW-573 C7): the weapon's [`Stable`]
/// brace tag, the GTW-392 [`TerrainBraced`] stair-brace decision, the GTW-549
/// per-item [`WeaponBraceBonus`] attachment term, and the GTW-543
/// [`EmplacementStability`] mounted-gun term.
///
/// One named bundle instead of four positional zero-identity parameters, so the
/// NEXT additive stability term is ONE new field here (with its zero-identity
/// [`Default`]) instead of a ~50-call-site positional sweep. Every field's
/// [`default`](Self::default) is its **zero identity** — the value under which the
/// score is byte-identical to a shot with no such term — so call sites spell only
/// the terms that are actually engaged, via struct-update:
///
/// ```ignore
/// StabilityTerms { stable: Stable::new(true), ..StabilityTerms::default() }
/// ```
///
/// The GTW-526 suppression term is NOT a field: it is resolved INSIDE
/// [`stability_for`](crate::aim::stability_for) from the shooter's own
/// [`Suppressed`](crate::ganger::Suppressed) state (a shooter property, not a
/// caller-resolved shot term). `Copy` — the fire path builds one value per round
/// and hands it to both `cone_for` and `stability_for`, keeping the cone width and
/// its recoil damping consistent by construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StabilityTerms {
    /// The weapon's unconditional-brace tag — zero identity `false`.
    pub stable:         Stable,
    /// The GTW-392 stair-brace decision — zero identity `false` (not braced).
    pub terrain_braced: TerrainBraced,
    /// The GTW-549 per-item attachment brace points — zero identity
    /// [`WeaponBraceBonus::none`] (no brace attachment).
    pub brace_bonus:    WeaponBraceBonus,
    /// The GTW-543 emplacement points — zero identity [`EmplacementStability::none`]
    /// (not manning a mounted gun).
    pub emplacement:    EmplacementStability,
}

impl Default for StabilityTerms {
    /// Every term at its **zero identity** — the all-identity bundle under which the
    /// §1a score is byte-identical to a shot that predates every seam (pinned by the
    /// `stability` tests' zero-identity-default case).
    fn default() -> Self {
        Self {
            stable:         Stable::new(false),
            terrain_braced: TerrainBraced::new(false),
            brace_bonus:    WeaponBraceBonus::none(),
            emplacement:    EmplacementStability::none(),
        }
    }
}

/// The **emplacement stability contribution** — the points a fixed emplacement
/// (bipod / tripod / mounted position) adds to the score (resolution.md §1a:
/// "+ an emplacement seam (no entities yet)").
///
/// The emplacement system is unbuilt, so callers pass [`EmplacementStability::none`]
/// (the zero/identity term) today; the seam exists so the
/// [`crate::stability::stability`] signature is complete and the term lands
/// without a later signature change. An INPUT to [`crate::stability::stability`],
/// distinct from the other three contributions. Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct EmplacementStability(f32);

impl EmplacementStability {
    /// Build an emplacement stability contribution from its point magnitude.
    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }

    /// The identity emplacement contribution — **zero** points, the value every
    /// caller passes today (no emplacement entities exist yet).
    #[must_use]
    pub const fn none() -> Self {
        Self(0.0)
    }
}

/// The **suppression stability contribution** — the points a
/// [`Suppressed`](crate::ganger::Suppressed) shooter's shakiness *subtracts* from
/// the score (GTW-526, child of GTW-41; `docs/combat/combat.md` "Suppression …
/// makes the target … a worse shot").
///
/// A pinned shooter shoots worse: the tunable suppression penalty is a **negative**
/// additive contribution lowering the stability score, so the cone-mult curve reads
/// a *higher* [`ConeMult`] (a **wider** dispersion cone). The seam mirrors
/// [`EmplacementStability`] EXACTLY — a fourth additive term summed by
/// [`crate::stability::stability`] with an [`SuppressionStability::none`] zero/identity
/// constructor callers pass when the shooter is un-suppressed, so an un-suppressed
/// shooter's score is **byte-identical** to before the seam existed.
///
/// The magnitude is the tunable
/// [`SuppressionStabilityPenalty`](crate::tuning::SuppressionStabilityPenalty) leaf,
/// resolved to `-penalty` by the composer ([`crate::aim::stability_for`]) when the
/// shooter carries [`Suppressed`](crate::ganger::Suppressed). An INPUT to
/// [`crate::stability::stability`], distinct from the other four contributions.
/// Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct SuppressionStability(f32);

impl SuppressionStability {
    /// Build a suppression stability contribution from its point magnitude.
    ///
    /// Callers pass a **negative** magnitude (the negated tuning penalty) so a
    /// suppressed shooter's contribution *lowers* the score; a positive magnitude
    /// would steady the shooter, the opposite of the design intent.
    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }

    /// The identity suppression contribution — **zero** points, the value every
    /// un-suppressed shooter passes (so the additive term vanishes and the score is
    /// byte-identical to a run without the seam).
    #[must_use]
    pub const fn none() -> Self {
        Self(0.0)
    }
}

/// The **normalised stability score** — the single 0–100 value the two curves
/// read off (resolution.md §1a: "Normalised over 100"). Built by
/// [`crate::stability::stability`] from the summed contributions, **clamped**
/// into `0..=100` so a degenerate over-100 sum can never read off the end of a
/// curve.
///
/// A domain value (the abstract steadiness of the shot, dimensionless — no
/// pixel), distinct from the contributions that feed it. Private inner + derived
/// [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct StabilityScore(f32);

impl StabilityScore {
    /// The bottom of the stability domain — a fully shaky shot.
    const MIN: f32 = 0.0;
    /// The top of the stability domain — a fully steady shot ("normalised over
    /// 100", resolution.md §1a). This is the domain ceiling of the *score axis*,
    /// not a tunable balance magnitude.
    pub(super) const MAX: f32 = 100.0;

    /// Build a stability score by **clamping** a raw contribution sum into the
    /// `0..=100` domain (resolution.md §1a normalisation) — so an over-100 sum
    /// saturates at 100 and a negative sum at 0, and the curve read can never run
    /// off either end.
    #[must_use]
    pub const fn clamped(raw: f32) -> Self {
        Self(raw.clamp(Self::MIN, Self::MAX))
    }
}

/// The **cone multiplier** — the `stability` term of `θ_cone` read off the
/// cone-mult curve (resolution.md §1a: "fed through a tuning curve → the cone
/// multiplier (steadier → narrower)"). A steadier score yields a smaller
/// multiplier, narrowing the dispersion cone.
///
/// One of the two distinct named outputs of [`crate::stability::stability`]
/// (never interchangeable with [`RecoilGrowth`], per no-bare-types rule 3). A
/// dimensionless angular multiplier — no pixel. Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ConeMult(f32);

impl ConeMult {
    /// Build a cone multiplier from its magnitude (a dimensionless angular scale).
    #[must_use]
    pub const fn new(mult: f32) -> Self {
        Self(mult)
    }
}

/// The **recoil-growth coefficient** — the score's SECOND curve output, read off
/// the recoil-growth curve (resolution.md §1a: round *i*'s axis tilts up by
/// `prior_shots × recoil_climb × recoil_growth`, so "a braced/prone shooter
/// climbs strictly less"). A steadier score yields a smaller coefficient, damping
/// how fast the muzzle walks up during a burst.
///
/// The other distinct named output of [`crate::stability::stability`] (never
/// interchangeable with [`ConeMult`]). A dimensionless climb damper — no pixel.
/// Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct RecoilGrowth(f32);

impl RecoilGrowth {
    /// Build a recoil-growth coefficient from its magnitude (a dimensionless climb
    /// damper).
    #[must_use]
    pub const fn new(coeff: f32) -> Self {
        Self(coeff)
    }
}

/// A raw **stability-curve output** — the value read off a
/// [`crate::tuning::StabilityCurve`] at a given [`StabilityScore`], before it is
/// given its axis-specific meaning.
///
/// Curve-axis-agnostic on purpose: [`crate::stability::stability`]'s curve read
/// interpolates a curve and returns this, then each caller wraps it into the
/// named output for that axis — a [`ConeMult`] off the cone-mult curve, a
/// [`RecoilGrowth`] off the recoil-growth curve. So the interpolation boundary
/// states "a curve output" in the type (`no-bare-types` rule 1) without
/// prejudging which curve produced it. Internal to this layer — private inner +
/// derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub(super) struct CurveOutput(f32);

impl CurveOutput {
    /// The **identity** curve output — `1.0`, no scaling. Returned for a degenerate
    /// (empty) curve so the read stays panic-free.
    pub(super) const IDENTITY: Self = Self(1.0);

    /// Build a curve output from the raw value read off a curve — the constructor
    /// the [`crate::stability::curve`] read uses so the inner stays private (house
    /// style: a newtype's inner is reached only through a ctor / [`Deref`]).
    pub(super) const fn new(output: f32) -> Self {
        Self(output)
    }
}
