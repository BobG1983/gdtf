//! The §1a **stability layer** — a continuous 0–100 stability score and its
//! two-curve read (`docs/combat/resolution.md` §1a + "What's pure math vs sim"
//! line 148: `stability(weapon_intrinsic, stance, brace, emplacement, …) →
//! (cone_mult, recoil_growth)`).
//!
//! Stability is a **continuous score derived from the situation**, not a binary
//! state (resolution.md §1a). [`stability`] sums four contributions — the
//! **weapon-intrinsic** contribution ([`WeaponStability`], a weapon-side input);
//! the **per-stance** contribution (prone 40 / kneel 25 / stand 10, read off
//! [`crate::tuning::StanceStability`]); the **automatic brace** contribution
//! (+30, [`crate::tuning::BraceContribution`]), applied EXACTLY when the faced
//! cell's [`crate::cover::CoverEntry::height_band`] satisfies the per-stance brace
//! min-height gate ([`crate::tuning::BraceMinHeight`]: prone↔LOW+, kneel↔MID+,
//! stand↔HIGH); and the **emplacement** seam ([`EmplacementStability`] — no
//! entities yet, a zero/identity term carried so the signature is complete) —
//! then **clamps/normalises** the sum into the `0..=100` domain
//! ([`StabilityScore`]) and reads **both** [`crate::tuning::StabilityCurves`] at
//! that score: the cone-mult curve ([`ConeMult`], steadier → narrower, < 1) and
//! the recoil-growth curve ([`RecoilGrowth`], steadier → climbs strictly less).
//!
//! The curve *form* (a clamped, piecewise-linear read over the authored sample
//! points) lives here in code; every *coefficient* — the contributions, the
//! gate, and both curves — comes from [`crate::tuning::CombatTuning`], so no
//! stability magnitude or band constant is hardcoded
//! (resolution.md §"Coefficients live in the combat-tuning data"). The whole
//! layer is **angular / dimensionless — zero pixels**: it never touches the
//! cubic-voxel metric, only the abstract score and the cover [`HeightBand`].
//!
//! The brace gate compares the faced cell's [`HeightBand`] **directly** against
//! the per-stance minimum band from tuning — [`crate::cover::band_for`] is NOT
//! called here (that helper maps a within-level [`crate::cover::BandFraction`] to
//! a [`HeightBand`] for the projectile clearance path, a different concern).

use bevy::prelude::Deref;

use crate::{
    cover::{CoverEntry, HeightBand},
    ganger::{Stance, StanceKind},
    tuning::{ConeStabilityTuning, StabilityCurve, StanceContribution},
};

/// The **weapon-intrinsic stability contribution** — the points a weapon's own
/// mechanics add to the 0–100 stability score (resolution.md §1a: "weapon
/// intrinsic + stance + …"). A heavy, well-balanced gun is steadier in the hands
/// than a light, snappy one before posture or bracing is considered.
///
/// A weapon-side INPUT to [`stability`] (a contribution, not a tuning
/// coefficient — it rides with the weapon), distinct from the per-stance and
/// brace contributions even though all four are summed into the same score.
/// Private inner + derived [`Deref`] (the crate's newtype house style).
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct WeaponStability(f32);

impl WeaponStability {
    /// Build a weapon-intrinsic stability contribution from its point magnitude.
    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }
}

/// The **emplacement stability contribution** — the points a fixed emplacement
/// (bipod / tripod / mounted position) adds to the score (resolution.md §1a:
/// "+ an emplacement seam (no entities yet)").
///
/// The emplacement system is unbuilt, so callers pass [`EmplacementStability::none`]
/// (the zero/identity term) today; the seam exists so the [`stability`] signature
/// is complete and the term lands without a later signature change. An INPUT to
/// [`stability`], distinct from the other three contributions. Private inner +
/// derived [`Deref`].
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

/// The **normalised stability score** — the single 0–100 value the two curves
/// read off (resolution.md §1a: "Normalised over 100"). Built by [`stability`]
/// from the summed contributions, **clamped** into `0..=100` so a degenerate
/// over-100 sum can never read off the end of a curve.
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
    const MAX: f32 = 100.0;

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
/// One of the two distinct named outputs of [`stability`] (never interchangeable
/// with [`RecoilGrowth`], per no-bare-types rule 3). A dimensionless angular
/// multiplier — no pixel. Private inner + derived [`Deref`].
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
/// The other distinct named output of [`stability`] (never interchangeable with
/// [`ConeMult`]). A dimensionless climb damper — no pixel. Private inner + derived
/// [`Deref`].
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

/// A raw **stability-curve output** — the value read off a [`StabilityCurve`] at a
/// given [`StabilityScore`], before it is given its axis-specific meaning.
///
/// Curve-axis-agnostic on purpose: [`read_curve`] interpolates a curve and returns
/// this, then each caller wraps it into the named output for that axis — a
/// [`ConeMult`] off the cone-mult curve, a [`RecoilGrowth`] off the recoil-growth
/// curve. So the interpolation boundary states "a curve output" in the type
/// (`no-bare-types` rule 1) without prejudging which curve produced it. Internal to
/// this layer — private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
struct CurveOutput(f32);

impl CurveOutput {
    /// The **identity** curve output — `1.0`, no scaling. Returned for a degenerate
    /// (empty) curve so the read stays panic-free.
    const IDENTITY: Self = Self(1.0);
}

/// The rank of a [`HeightBand`] on the LOW < MID < HIGH ladder — the orderable
/// view of the cover-height bands the brace gate compares.
///
/// [`HeightBand`] is a plain three-variant enum (no `Ord`), so the gate needs an
/// ordering: this maps each band to its position so "the faced band reaches the
/// stance's minimum band" is a `>=` over the ranks. It is **not** a hardcoded
/// band number — the *thresholds* still come from [`crate::tuning::BraceMinHeight`];
/// this only orders the enum the doc already orders ("LOW+ / MID+ / HIGH").
const fn band_rank(band: HeightBand) -> u8 {
    match band {
        HeightBand::Low => 0,
        HeightBand::Mid => 1,
        HeightBand::High => 2,
    }
}

/// The per-stance **minimum brace band** the faced cover must reach for this
/// stance, read off the tuning gate (resolution.md §1a: prone↔LOW+, kneel↔MID+,
/// stand↔HIGH). No band literal lives here — the threshold band is taken from
/// `tuning.brace_min_height`.
const fn brace_min_band(stance: StanceKind, tuning: &ConeStabilityTuning) -> HeightBand {
    match stance {
        StanceKind::Prone => tuning.brace_min_height.prone,
        StanceKind::Crouching => tuning.brace_min_height.kneel,
        StanceKind::Standing => tuning.brace_min_height.stand,
    }
}

/// The per-stance **stability contribution** for this stance, read off the tuning
/// triple (resolution.md §1a: prone 40 / kneel 25 / stand 10). No magnitude lives
/// here — the points are taken from `tuning.stance_stability`. The return states
/// its domain meaning in the type ([`StanceContribution`]), per `no-bare-types`.
const fn stance_contribution(
    stance: StanceKind,
    tuning: &ConeStabilityTuning,
) -> StanceContribution {
    match stance {
        StanceKind::Prone => tuning.stance_stability.prone,
        StanceKind::Crouching => tuning.stance_stability.kneel,
        StanceKind::Standing => tuning.stance_stability.stand,
    }
}

/// Whether the **automatic brace** engages for `stance` against the `faced` cell —
/// `true` EXACTLY when there is cover in the faced cell **and** its
/// [`HeightBand`], read **directly** off the [`CoverEntry`], reaches the stance's
/// minimum brace band from tuning (resolution.md §1a brace gate). No cover faced
/// (`None`) never braces; [`crate::cover::band_for`] is not consulted.
const fn brace_engages(
    stance: StanceKind,
    faced: Option<&CoverEntry>,
    tuning: &ConeStabilityTuning,
) -> bool {
    let Some(entry) = faced else {
        return false;
    };
    band_rank(entry.height_band) >= band_rank(brace_min_band(stance, tuning))
}

/// Read a [`StabilityCurve`] at `score` by a **clamped, piecewise-linear**
/// interpolation over its authored sample points — the curve *form*
/// (resolution.md §1a: "fed through a tuning curve"; the points are tuning, the
/// interpolation is code).
///
/// The score is already clamped to the curve's `0..=100` domain ([`StabilityScore`]),
/// and the read is additionally **clamped to the authored endpoints**: a score at
/// or below the first point returns the first point's output, a score at or above
/// the last returns the last's, and a score between two points linearly
/// interpolates their outputs. An **empty** curve (no authored points — a
/// degenerate tuning) returns the identity `1.0` (no scaling) rather than panicking,
/// keeping the layer panic-free (resolution.md C6 "no unwrap/expect/panic").
///
/// Returns the axis-agnostic [`CurveOutput`]; the caller wraps it into the named
/// output for whichever curve was read (`no-bare-types`).
fn read_curve(curve: &StabilityCurve, score: StabilityScore) -> CurveOutput {
    let s = *score;
    let points: &[_] = curve;
    let Some(first) = points.first() else {
        // Degenerate empty curve: identity scaling, never a panic.
        return CurveOutput::IDENTITY;
    };
    // At or below the first sample → the first output (left clamp).
    if s <= *first.score {
        return CurveOutput(*first.output);
    }
    // Walk adjacent pairs; the score sits in exactly one span (points are authored
    // in ascending score order — resolution.md §1a curve definition).
    for pair in points.windows(2) {
        let [lo, hi] = pair else {
            continue;
        };
        if s <= *hi.score {
            let span = *hi.score - *lo.score;
            // Coincident scores (zero span) → take the higher point's output rather
            // than dividing by zero.
            if span <= 0.0 {
                return CurveOutput(*hi.output);
            }
            let t = (s - *lo.score) / span;
            return CurveOutput(t.mul_add(*hi.output - *lo.output, *lo.output));
        }
    }
    // Above the last sample → the last output (right clamp). `last()` is `Some`
    // because `first()` was; fall back to the identity if somehow absent.
    points
        .last()
        .map_or(CurveOutput::IDENTITY, |p| CurveOutput(*p.output))
}

/// Compute the §1a stability score and its two-curve read for a shooter facing a
/// (possibly empty) cover cell (resolution.md §1a; "What's pure math vs sim" line
/// 148: `stability(weapon_intrinsic, stance, brace, emplacement, …) → (cone_mult,
/// recoil_growth)`).
///
/// Sums the four contributions — the `weapon_intrinsic` points, the per-stance
/// contribution, the automatic brace contribution (applied EXACTLY when `faced`'s
/// cover [`HeightBand`] satisfies `stance`'s min-height gate), and the
/// `emplacement` seam — then
/// **clamps/normalises** the sum into the `0..=100` [`StabilityScore`] domain and
/// reads **both** tuning curves at that score, returning the named
/// `(cone_mult, recoil_growth)` pair. A steadier situation yields a higher score,
/// hence a smaller [`ConeMult`] (narrower cone) and a smaller [`RecoilGrowth`]
/// (less climb).
///
/// `faced` is the [`CoverEntry`] of the cell the shooter faces (the brace gate
/// reads its `height_band` directly), or `None` when no cover is faced — in which
/// case the brace contribution is withheld. Every coefficient and both curves come
/// from `tuning`; nothing tunable is hardcoded. Angular / dimensionless — zero
/// pixels.
#[must_use]
pub fn stability(
    weapon_intrinsic: WeaponStability,
    stance: Stance,
    faced: Option<&CoverEntry>,
    emplacement: EmplacementStability,
    tuning: &ConeStabilityTuning,
) -> (ConeMult, RecoilGrowth) {
    let posture = *stance;

    // Sum the four §1a contributions: weapon intrinsic + per-stance + brace (only
    // when the faced cover satisfies the per-stance gate) + the emplacement seam.
    let brace = if brace_engages(posture, faced, tuning) {
        *tuning.brace_contribution
    } else {
        0.0
    };
    let raw = *weapon_intrinsic + *stance_contribution(posture, tuning) + brace + *emplacement;

    // Normalise/clamp into the 0..=100 score domain BEFORE the curve read, then
    // read BOTH curves at that one score, wrapping each axis-agnostic CurveOutput
    // into its named output.
    let score = StabilityScore::clamped(raw);
    let cone_mult = read_curve(&tuning.stability_curves.cone_mult, score);
    let recoil_growth = read_curve(&tuning.stability_curves.recoil_growth, score);
    (ConeMult::new(*cone_mult), RecoilGrowth::new(*recoil_growth))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        armor::{ArmorHardness, ArmorProtection},
        cover::CoverHp,
        ganger::Stance,
    };

    /// An arbitrary faced-cover entry at `band` — NOT shipped magnitudes; the
    /// stability layer only reads `height_band`, so the HP/armor are filler.
    fn faced_cover(band: HeightBand) -> CoverEntry {
        CoverEntry::seeded(
            CoverHp::new(10),
            band,
            ArmorProtection::new(1),
            ArmorHardness::new(1),
        )
    }

    /// C1 — `stability(...)` returns BOTH named outputs computed from the four
    /// contributions, normalised over 100, read off the two tuning curves. A
    /// relation, not a magnitude: a fully steady situation (prone + braced on a
    /// HIGH wall) must produce a finite `cone_mult` and `recoil_growth`, and (with
    /// the default curves' steadier-is-smaller shape) a smaller `cone_mult` than a
    /// fully shaky one. Value-agnostic on the actual numbers.
    #[test]
    fn stability_produces_both_named_outputs() {
        let tuning = ConeStabilityTuning::default();
        let wall = faced_cover(HeightBand::High);

        // Steadiest: prone, braced on a HIGH wall (satisfies the prone gate, LOW+).
        let (steady_cone, steady_recoil) = stability(
            WeaponStability::new(0.0),
            Stance::new(StanceKind::Prone),
            Some(&wall),
            EmplacementStability::none(),
            &tuning,
        );
        // Shakiest: standing, no cover faced (no brace), no weapon/emplacement help.
        let (shaky_cone, shaky_recoil) = stability(
            WeaponStability::new(0.0),
            Stance::new(StanceKind::Standing),
            None,
            EmplacementStability::none(),
            &tuning,
        );

        // Both outputs are produced and finite.
        assert!((*steady_cone).is_finite() && (*steady_recoil).is_finite());
        assert!((*shaky_cone).is_finite() && (*shaky_recoil).is_finite());
        // Steadier yields a narrower cone and less climb (relation, not magnitude).
        assert!(
            *steady_cone < *shaky_cone,
            "a steadier situation must yield a narrower cone_mult",
        );
        assert!(
            *steady_recoil < *shaky_recoil,
            "a steadier situation must yield less recoil_growth",
        );
    }

    /// C2 — the auto-brace contribution is applied EXACTLY when the faced cell's
    /// `CoverEntry.height_band` satisfies the per-stance gate, and withheld
    /// otherwise. For each stance, pair a SATISFYING faced band with a
    /// NON-satisfying one (both read directly off the `CoverEntry`) and assert
    /// brace-on is steadier (lower `cone_mult`) than brace-off. No band literal — the
    /// satisfying/failing bands are derived from the tuning gate itself.
    #[test]
    fn brace_applied_exactly_when_faced_band_satisfies_gate() {
        let tuning = ConeStabilityTuning::default();

        // (stance, a band that SATISFIES its gate, a band that FAILS it).
        // Derived from the doc gate prone↔LOW+, kneel↔MID+, stand↔HIGH: the gate
        // band itself satisfies; the band one rank below it fails (stand's gate is
        // HIGH, so MID fails; kneel's is MID, so LOW fails; prone's is LOW — nothing
        // is below LOW, so prone's "fail" is the no-cover-faced case, asserted
        // separately below).
        let cases = [
            (StanceKind::Standing, HeightBand::High, HeightBand::Mid),
            (StanceKind::Crouching, HeightBand::Mid, HeightBand::Low),
        ];
        for (kind, satisfying, failing) in cases {
            let sat = faced_cover(satisfying);
            let fail = faced_cover(failing);
            let (braced, _) = stability(
                WeaponStability::new(0.0),
                Stance::new(kind),
                Some(&sat),
                EmplacementStability::none(),
                &tuning,
            );
            let (unbraced, _) = stability(
                WeaponStability::new(0.0),
                Stance::new(kind),
                Some(&fail),
                EmplacementStability::none(),
                &tuning,
            );
            assert!(
                *braced < *unbraced,
                "{kind:?}: a satisfying faced band must brace (steadier, lower cone_mult) vs a failing one",
            );
        }

        // Prone's gate is LOW+, so a LOW wall satisfies it; no cover faced does not.
        let low = faced_cover(HeightBand::Low);
        let (prone_braced, _) = stability(
            WeaponStability::new(0.0),
            Stance::new(StanceKind::Prone),
            Some(&low),
            EmplacementStability::none(),
            &tuning,
        );
        let (prone_unbraced, _) = stability(
            WeaponStability::new(0.0),
            Stance::new(StanceKind::Prone),
            None,
            EmplacementStability::none(),
            &tuning,
        );
        assert!(
            *prone_braced < *prone_unbraced,
            "prone braced on a LOW wall must be steadier than prone facing no cover",
        );
    }

    /// C3 — steadier postures yield a steadier score: prone < kneel < stand in
    /// `cone_mult` for the same weapon and NO brace (none faced). A
    /// monotonic-relation test over the three stances — ordering only, never
    /// magnitudes.
    #[test]
    fn steadier_stance_yields_narrower_cone() {
        let tuning = ConeStabilityTuning::default();
        let cone = |kind| {
            stability(
                WeaponStability::new(0.0),
                Stance::new(kind),
                None,
                EmplacementStability::none(),
                &tuning,
            )
            .0
        };
        let prone = *cone(StanceKind::Prone);
        let kneel = *cone(StanceKind::Crouching);
        let stand = *cone(StanceKind::Standing);
        assert!(
            prone < kneel && kneel < stand,
            "prone < kneel < stand in cone_mult (steadier → narrower): {prone} {kneel} {stand}",
        );
    }

    /// C4 — `recoil_growth` is the score's SECOND curve output, and a steadier
    /// score yields strictly-LESS climb: a braced/prone shooter's `recoil_growth`
    /// is strictly below a standing/un-braced one's (ordering, not magnitude).
    #[test]
    fn steadier_score_yields_strictly_less_recoil_growth() {
        let tuning = ConeStabilityTuning::default();
        let wall = faced_cover(HeightBand::High);

        let (_, braced_prone) = stability(
            WeaponStability::new(0.0),
            Stance::new(StanceKind::Prone),
            Some(&wall),
            EmplacementStability::none(),
            &tuning,
        );
        let (_, standing_unbraced) = stability(
            WeaponStability::new(0.0),
            Stance::new(StanceKind::Standing),
            None,
            EmplacementStability::none(),
            &tuning,
        );
        assert!(
            *braced_prone < *standing_unbraced,
            "a braced/prone shooter must climb strictly less than a standing/un-braced one",
        );
    }

    /// C5 — the score is clamped/normalised to 0–100 BEFORE the curve read: a
    /// degenerate over-100 contribution sum does not read off the curve's end or
    /// panic. A maximal contribution sum (huge weapon + emplacement + prone + brace)
    /// must produce the SAME outputs as a sum that exactly reaches 100, proving the
    /// clamp (and never a panic / NaN).
    #[test]
    fn over_100_sum_clamps_and_does_not_run_off_the_curve() {
        let tuning = ConeStabilityTuning::default();
        let wall = faced_cover(HeightBand::High);

        // A wildly over-100 raw sum.
        let (over_cone, over_recoil) = stability(
            WeaponStability::new(10_000.0),
            Stance::new(StanceKind::Prone),
            Some(&wall),
            EmplacementStability::new(10_000.0),
            &tuning,
        );
        // A sum that lands exactly at the score ceiling (100) via the weapon term
        // alone (prone + brace withheld here: standing, no cover).
        let (ceil_cone, ceil_recoil) = stability(
            WeaponStability::new(StabilityScore::MAX),
            Stance::new(StanceKind::Standing),
            None,
            EmplacementStability::none(),
            &tuning,
        );

        assert!((*over_cone).is_finite() && (*over_recoil).is_finite());
        // Both clamp to the score ceiling, so the curve reads are identical — the
        // over-100 sum did not run off the curve's end.
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

    /// The output newtypes' derived [`Deref`] reaches their inner value, and the
    /// two outputs are DISTINCT types (no-bare-types rule 3). Built from arbitrary
    /// literals — pins the Deref mechanism + target type, not a magnitude.
    #[test]
    fn output_newtypes_deref_to_inner() {
        assert_eq!((*ConeMult::new(0.7)).to_bits(), 0.7_f32.to_bits());
        assert_eq!((*RecoilGrowth::new(0.3)).to_bits(), 0.3_f32.to_bits());
        assert_eq!((*WeaponStability::new(5.0)).to_bits(), 5.0_f32.to_bits());
        assert_eq!(
            (*EmplacementStability::new(8.0)).to_bits(),
            8.0_f32.to_bits()
        );
        assert_eq!((*EmplacementStability::none()).to_bits(), 0.0_f32.to_bits());
        assert_eq!(
            (*StabilityScore::clamped(50.0)).to_bits(),
            50.0_f32.to_bits()
        );
    }
}
