//! The §1a **score composition** — [`stability`], which sums the three
//! contributions, normalises into the `0..=100` [`StabilityScore`] domain, and
//! reads both tuning curves to return the `(cone_mult, recoil_growth)` pair.

use crate::{
    cover::CoverEntry,
    ganger::Stance,
    stability::{
        curve::read_curve,
        gate::{brace_engages, stance_contribution},
        types::{ConeMult, RecoilGrowth, StabilityScore, StabilityTerms, SuppressionStability},
    },
    tuning::ConeStabilityTuning,
};

/// Compute the §1a stability score and its two-curve read for a shooter facing a
/// (possibly empty) cover cell (resolution.md §1a; "What's pure math vs sim" line
/// 148: `stability(stance, brace, emplacement, …) → (cone_mult, recoil_growth)`).
///
/// Sums the contributions — the per-stance contribution, the automatic
/// brace contribution (applied when `faced`'s cover [`crate::cover::HeightBand`]
/// satisfies `stance`'s min-height gate **OR** the weapon is `stable` **OR** the
/// shooter has a [`TerrainBraced`](crate::stability::TerrainBraced) stair brace,
/// GTW-392), the GTW-543 emplacement seam, the GTW-526 `suppression` seam (a
/// **negative** term when the shooter is [`Suppressed`](crate::ganger::Suppressed),
/// zero otherwise), and the GTW-549 per-item brace seam (a **positive** term when
/// the weapon carries a data-driven
/// [`WeaponBraceBonus`](crate::effects::attachments::WeaponBraceBonus) attachment, zero otherwise)
/// — then **clamps/normalises** the sum into the `0..=100` [`StabilityScore`]
/// domain and reads **both** tuning curves at that score, returning the named
/// `(cone_mult, recoil_growth)` pair. A steadier situation yields a higher score,
/// hence a smaller [`ConeMult`] (narrower cone) and a smaller [`RecoilGrowth`]
/// (less climb); a suppressed shooter's negative term LOWERS the score, WIDENING
/// the cone. There is **no** weapon-intrinsic stability *points* term — a weapon's
/// only contribution is the boolean `stable` tag, which engages the brace
/// unconditionally.
///
/// `terms` bundles the four caller-resolved zero-identity terms ([`StabilityTerms`]
/// — GTW-573 C7): the weapon's `stable` tag, the GTW-392 terrain-brace decision, the
/// GTW-549 per-item brace points, and the GTW-543 emplacement points. Each is
/// pure-additive (or gate-only), so a term at its zero-identity
/// [`default`](StabilityTerms::default) leaves the score byte-identical to a shot
/// without that seam — the zero-identity-default test pins it. `faced` is the
/// [`CoverEntry`] of the cell the shooter faces (the brace gate reads its
/// `height_band` directly), or `None` when no cover is faced — in which case the
/// brace contribution is withheld for a non-stable, non-terrain-braced weapon.
/// `suppression` is the GTW-526 additive suppression term —
/// [`SuppressionStability::none`] (zero, the identity) for an un-suppressed shooter,
/// or the negated tunable penalty when the shooter is
/// [`Suppressed`](crate::ganger::Suppressed) (a shooter property, resolved by the
/// composer — not a caller term, so it stays its own parameter). Every coefficient
/// and both curves come from `tuning`; nothing tunable is hardcoded. Angular /
/// dimensionless — zero pixels.
#[must_use]
pub fn stability(
    terms: StabilityTerms,
    stance: Stance,
    faced: Option<&CoverEntry>,
    suppression: SuppressionStability,
    tuning: &ConeStabilityTuning,
) -> (ConeMult, RecoilGrowth) {
    let posture = *stance;

    // Sum the §1a contributions: per-stance + brace (when the faced cover
    // satisfies the per-stance gate OR the weapon is stable OR terrain-braced) +
    // the emplacement seam + the GTW-526 suppression seam (a negative term when the
    // shooter is Suppressed, zero otherwise) + the GTW-549 per-item brace seam (a positive
    // term carried by a data-driven `Stability` attachment's `WeaponBraceBonus`, zero
    // otherwise). The brace_engages bool is OR-combined so all three UNCONDITIONAL brace
    // sources produce exactly ONE brace_contribution quantum — never a sum; the per-item
    // `brace_bonus` is a SEPARATE graduated additive term (a magnitude, not a boolean gate).
    // suppression and brace_bonus are BOTH pure-additive identity (0.0) when absent, so an
    // un-suppressed weapon with no brace attachment has a raw sum — and thus its clamped
    // score and both curve reads — byte-identical to before either seam landed.
    let brace = if brace_engages(terms.stable, terms.terrain_braced, posture, faced, tuning) {
        *tuning.brace_contribution
    } else {
        0.0
    };
    let raw = *stance_contribution(posture, tuning)
        + brace
        + *terms.emplacement
        + *suppression
        + *terms.brace_bonus;

    // Normalise/clamp into the 0..=100 score domain BEFORE the curve read, then
    // read BOTH curves at that one score, wrapping each axis-agnostic CurveOutput
    // into its named output.
    let score = StabilityScore::clamped(raw);
    let cone_mult = read_curve(&tuning.stability_curves.cone_mult, score);
    let recoil_growth = read_curve(&tuning.stability_curves.recoil_growth, score);
    (ConeMult::new(*cone_mult), RecoilGrowth::new(*recoil_growth))
}
