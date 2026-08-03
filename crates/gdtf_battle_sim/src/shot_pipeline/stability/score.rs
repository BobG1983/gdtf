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

#[must_use]
pub fn stability(
    terms: StabilityTerms,
    stance: Stance,
    faced: Option<&CoverEntry>,
    suppression: SuppressionStability,
    tuning: &ConeStabilityTuning,
) -> (ConeMult, RecoilGrowth) {
    let posture = *stance;

    let brace = if *brace_engages(terms.stable, terms.terrain_braced, posture, faced, tuning) {
        *tuning.brace_contribution
    } else {
        0.0
    };
    let raw = *stance_contribution(posture, tuning)
        + brace
        + *terms.emplacement
        + *suppression
        + *terms.brace_bonus;

    let score = StabilityScore::clamped(raw);
    let cone_mult = read_curve(&tuning.stability_curves.cone_mult, score);
    let recoil_growth = read_curve(&tuning.stability_curves.recoil_growth, score);
    (ConeMult::new(*cone_mult), RecoilGrowth::new(*recoil_growth))
}
