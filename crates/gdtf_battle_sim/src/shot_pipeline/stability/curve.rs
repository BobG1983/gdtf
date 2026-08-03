//! Interpolate authored stability curves.

use crate::{
    stability::types::{CurveOutput, StabilityScore},
    tuning::StabilityCurve,
};

/// Read a curve at the given stability score (linear interpolation between points).
pub(super) fn read_curve(curve: &StabilityCurve, score: StabilityScore) -> CurveOutput {
    let s = *score;
    let points: &[_] = curve;
    let Some(first) = points.first() else {
        return CurveOutput::IDENTITY;
    };
    if s <= *first.score {
        return CurveOutput::new(*first.output);
    }
    for pair in points.windows(2) {
        let [lo, hi] = pair else {
            continue;
        };
        if s <= *hi.score {
            let span = *hi.score - *lo.score;
            if span <= 0.0 {
                return CurveOutput::new(*hi.output);
            }
            let t = (s - *lo.score) / span;
            return CurveOutput::new(t.mul_add(*hi.output - *lo.output, *lo.output));
        }
    }
    points
        .last()
        .map_or(CurveOutput::IDENTITY, |p| CurveOutput::new(*p.output))
}
