//! The §1a **score composition** — [`stability`], which sums the three
//! contributions, normalises into the `0..=100` [`StabilityScore`] domain, and
//! reads both tuning curves to return the `(cone_mult, recoil_growth)` pair.

use crate::{
    cover::CoverEntry,
    ganger::Stance,
    stability::{
        curve::read_curve,
        gate::{brace_engages, stance_contribution},
        types::{ConeMult, EmplacementStability, RecoilGrowth, StabilityScore},
    },
    tuning::ConeStabilityTuning,
    weapon::Stable,
};

/// Compute the §1a stability score and its two-curve read for a shooter facing a
/// (possibly empty) cover cell (resolution.md §1a; "What's pure math vs sim" line
/// 148: `stability(stance, brace, emplacement, …) → (cone_mult, recoil_growth)`).
///
/// Sums the three contributions — the per-stance contribution, the automatic
/// brace contribution (applied when `faced`'s cover [`crate::cover::HeightBand`]
/// satisfies `stance`'s min-height gate **OR** the weapon is `stable`), and the
/// `emplacement` seam — then **clamps/normalises** the sum into the `0..=100`
/// [`StabilityScore`] domain and reads **both** tuning curves at that score,
/// returning the named `(cone_mult, recoil_growth)` pair. A steadier situation
/// yields a higher score, hence a smaller [`ConeMult`] (narrower cone) and a
/// smaller [`RecoilGrowth`] (less climb). There is **no** weapon-intrinsic
/// stability *points* term — a weapon's only contribution is the boolean `stable`
/// tag, which engages the brace unconditionally.
///
/// `stable` is the weapon's [`crate::weapon::Stable`] tag: a stable weapon braces
/// regardless of faced cover or stance. `faced` is the [`CoverEntry`] of the cell
/// the shooter faces (the brace gate reads its `height_band` directly), or `None`
/// when no cover is faced — in which case the brace contribution is withheld for a
/// non-stable weapon. Every coefficient and both curves come from `tuning`;
/// nothing tunable is hardcoded. Angular / dimensionless — zero pixels.
#[must_use]
pub fn stability(
    stable: Stable,
    stance: Stance,
    faced: Option<&CoverEntry>,
    emplacement: EmplacementStability,
    tuning: &ConeStabilityTuning,
) -> (ConeMult, RecoilGrowth) {
    let posture = *stance;

    // Sum the three §1a contributions: per-stance + brace (when the faced cover
    // satisfies the per-stance gate OR the weapon is stable) + the emplacement seam.
    let brace = if brace_engages(stable, posture, faced, tuning) {
        *tuning.brace_contribution
    } else {
        0.0
    };
    let raw = *stance_contribution(posture, tuning) + brace + *emplacement;

    // Normalise/clamp into the 0..=100 score domain BEFORE the curve read, then
    // read BOTH curves at that one score, wrapping each axis-agnostic CurveOutput
    // into its named output.
    let score = StabilityScore::clamped(raw);
    let cone_mult = read_curve(&tuning.stability_curves.cone_mult, score);
    let recoil_growth = read_curve(&tuning.stability_curves.recoil_growth, score);
    (ConeMult::new(*cone_mult), RecoilGrowth::new(*recoil_growth))
}
