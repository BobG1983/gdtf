//! The §1a **curve read** — the clamped, piecewise-linear interpolation over a
//! [`StabilityCurve`]'s authored sample points (the curve *form*; the points are
//! tuning, the interpolation is code).

use crate::{
    stability::types::{CurveOutput, StabilityScore},
    tuning::StabilityCurve,
};

/// Read a [`StabilityCurve`] at `score` by a **clamped, piecewise-linear**
/// interpolation over its authored sample points — the curve *form*
/// (resolution.md §1a: "fed through a tuning curve"; the points are tuning, the
/// interpolation is code).
///
/// The score is already clamped to the curve's `0..=100` domain
/// ([`StabilityScore`]), and the read is additionally **clamped to the authored
/// endpoints**: a score at or below the first point returns the first point's
/// output, a score at or above the last returns the last's, and a score between
/// two points linearly interpolates their outputs. An **empty** curve (no authored
/// points — a degenerate tuning) returns the identity `1.0` (no scaling) rather
/// than panicking, keeping the layer panic-free (resolution.md C6 "no
/// unwrap/expect/panic").
///
/// Returns the axis-agnostic [`CurveOutput`]; the caller wraps it into the named
/// output for whichever curve was read (`no-bare-types`).
pub(super) fn read_curve(curve: &StabilityCurve, score: StabilityScore) -> CurveOutput {
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
