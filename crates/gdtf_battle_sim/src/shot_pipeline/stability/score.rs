//! The §1a **score composition** — [`stability`], which sums the three
//! contributions, normalises into the `0..=100` [`StabilityScore`] domain, and
//! reads both tuning curves to return the `(cone_mult, recoil_growth)` pair.

use crate::{
    cover::CoverEntry,
    ganger::Stance,
    stability::{
        TerrainBraced,
        curve::read_curve,
        gate::{brace_engages, stance_contribution},
        types::{
            ConeMult, EmplacementStability, RecoilGrowth, SightStability, StabilityScore,
            SuppressionStability,
        },
    },
    tuning::ConeStabilityTuning,
    weapon::Stable,
};

/// Compute the §1a stability score and its two-curve read for a shooter facing a
/// (possibly empty) cover cell (resolution.md §1a; "What's pure math vs sim" line
/// 148: `stability(stance, brace, emplacement, …) → (cone_mult, recoil_growth)`).
///
/// Sums the contributions — the per-stance contribution, the automatic
/// brace contribution (applied when `faced`'s cover [`crate::cover::HeightBand`]
/// satisfies `stance`'s min-height gate **OR** the weapon is `stable` **OR** the
/// shooter has a [`TerrainBraced`] stair brace, GTW-392), the `emplacement` seam,
/// the GTW-526 `suppression` seam (a **negative** term when the shooter is
/// [`Suppressed`](crate::ganger::Suppressed), zero otherwise), and the GTW-542 `sight`
/// seam (a **positive** term when the weapon is [`Scoped`](crate::weapon::Scoped),
/// zero otherwise) — then
/// **clamps/normalises** the sum into the `0..=100` [`StabilityScore`] domain and
/// reads **both** tuning curves at that score, returning the named
/// `(cone_mult, recoil_growth)` pair. A steadier situation yields a higher score,
/// hence a smaller [`ConeMult`] (narrower cone) and a smaller [`RecoilGrowth`]
/// (less climb); a suppressed shooter's negative term LOWERS the score, WIDENING
/// the cone. There is **no** weapon-intrinsic stability *points* term — a weapon's
/// only contribution is the boolean `stable` tag, which engages the brace
/// unconditionally.
///
/// `stable` is the weapon's [`crate::weapon::Stable`] tag: a stable weapon braces
/// regardless of faced cover or stance. `terrain_braced` is the GTW-392
/// [`TerrainBraced`] stair-brace decision computed at the fire call site from the
/// live grids. `faced` is the [`CoverEntry`] of the cell the shooter faces (the
/// brace gate reads its `height_band` directly), or `None` when no cover is faced
/// — in which case the brace contribution is withheld for a non-stable,
/// non-terrain-braced weapon. `suppression` is the GTW-526 additive suppression
/// term — [`SuppressionStability::none`] (zero, the identity) for an un-suppressed
/// shooter, or the negated tunable penalty when the shooter is
/// [`Suppressed`](crate::ganger::Suppressed); it is pure-additive, so an
/// un-suppressed shooter's score is byte-identical to before the seam existed.
/// `sight` is the GTW-542 additive sight term — [`SightStability::none`] (zero, the
/// identity) for an un-sighted weapon, or the tunable sight bonus when the weapon is
/// [`Scoped`](crate::weapon::Scoped); like `suppression` it is pure-additive, so an
/// un-sighted weapon's score is byte-identical to before that seam existed.
/// Every coefficient and both curves come from `tuning`; nothing tunable is
/// hardcoded. Angular / dimensionless — zero pixels.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "the §1a stability verb sums the FIVE distinct additive contributions the design \
              specifies — the weapon `stable` tag, the GTW-392 terrain brace, the shooter \
              stance, the faced cover, the emplacement seam, the GTW-526 suppression seam, and \
              the GTW-542 sight seam — plus the tuning; each is a distinct named domain input \
              (no-bare-types), and bundling them would hide which contribution feeds the sum"
)]
pub fn stability(
    stable: Stable,
    terrain_braced: TerrainBraced,
    stance: Stance,
    faced: Option<&CoverEntry>,
    emplacement: EmplacementStability,
    suppression: SuppressionStability,
    sight: SightStability,
    tuning: &ConeStabilityTuning,
) -> (ConeMult, RecoilGrowth) {
    let posture = *stance;

    // Sum the §1a contributions: per-stance + brace (when the faced cover
    // satisfies the per-stance gate OR the weapon is stable OR terrain-braced) +
    // the emplacement seam + the GTW-526 suppression seam (a negative term when the
    // shooter is Suppressed, zero otherwise) + the GTW-542 sight seam (a positive term
    // when the weapon is Scoped, zero otherwise). The brace_engages bool is OR-combined
    // so all three brace sources produce exactly ONE brace_contribution quantum — never a
    // sum. suppression and sight are BOTH pure-additive identity (0.0) when absent, so an
    // un-suppressed + un-sighted shot's raw sum — and thus its clamped score and both
    // curve reads — is byte-identical to before either seam landed.
    let brace = if brace_engages(stable, terrain_braced, posture, faced, tuning) {
        *tuning.brace_contribution
    } else {
        0.0
    };
    let raw = *stance_contribution(posture, tuning) + brace + *emplacement + *suppression + *sight;

    // Normalise/clamp into the 0..=100 score domain BEFORE the curve read, then
    // read BOTH curves at that one score, wrapping each axis-agnostic CurveOutput
    // into its named output.
    let score = StabilityScore::clamped(raw);
    let cone_mult = read_curve(&tuning.stability_curves.cone_mult, score);
    let recoil_growth = read_curve(&tuning.stability_curves.recoil_growth, score);
    (ConeMult::new(*cone_mult), RecoilGrowth::new(*recoil_growth))
}
