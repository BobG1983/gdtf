//! The SAFE tunable-range draw — [`uniform_or_midpoint`] (GTW-640 / GTW-644).
//!
//! Combat ranges are built from TUNABLE data at runtime: hot-reloadable `.ron`
//! tunings (and per-entity stats such as Luck) form the bounds of the sim's
//! `random_range` draws, so every such range is effectively attacker-controlled —
//! a documented-legal tuning edit (e.g. a [`FightVariance`](crate::tuning::FightVariance)
//! of `0.0`, GTW-640) can make a range EMPTY or INVERTED mid-session. `rand`'s
//! `random_range` PANICS on an empty range ("cannot sample empty range"), and
//! guarding by SKIPPING the draw (the old §6 roll-term guard, GTW-644) silently
//! breaks seeded determinism instead: the stream's cursor stops advancing the same
//! number of steps per call, so the same seed produces divergent downstream draws
//! depending on whether a degenerate range was hit.
//!
//! [`uniform_or_midpoint`] is the ONE systemic answer to both failure shapes:
//! never panic, never skip a draw. Every stream type exposes it as
//! `random_range_or_midpoint` (stamped by the `impl_sim_stream!` macro in
//! `streams`); the logic lives here, once.

use core::ops::Range;

use bevy::prelude::Deref;
use rand::{Rng, RngExt};

/// A value **drawn** from a tunable `f32` range — uniform when the range is live, its
/// bounds' midpoint when the range is degenerate (empty or inverted).
///
/// A named newtype over `f32` (no-bare-types: a sampled range value is a domain value,
/// not a bare float). Private inner + derived [`Deref`]; the per-stream
/// `random_range_or_midpoint` wrapper reads it back as the bare `f32` its callers expect,
/// so the byte-exact seeded-replay draw order is unchanged.
#[derive(Deref, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub(super) struct RangeSample(f32);

impl RangeSample {
    /// Wrap a drawn range value.
    #[must_use]
    const fn new(value: f32) -> Self {
        Self(value)
    }
}

/// Draw uniformly from a tunable-driven `f32` range — NEVER panicking and ALWAYS
/// consuming exactly one draw, even when the range is empty or inverted.
///
/// - A LIVE range (`start < end`) draws byte-identically to a plain
///   `random_range` — existing seeded replays over valid ranges are untouched.
/// - A DEGENERATE range (`start >= end` — empty or inverted) still consumes
///   exactly one uniform draw (burned on the never-empty unit range, which
///   advances rand 0.10's `UniformFloat` stream exactly as far as any `f32`
///   range draw: one word, no rejection loop) and returns the bounds' MIDPOINT —
///   the value the interval degenerates onto (the §7 band `[1 − v, 1 + v]` with
///   `v <= 0.0` collapses to factor `1.0`; a `lo >= hi` §6 roll collapses to
///   `(lo + hi) / 2`).
///
/// # Draw-count stability — the invariant, not an optimization
///
/// One call = one draw, unconditionally. Hot-reloadable tunings make every
/// tunable range attacker-controlled at runtime, so the stream cursor MUST
/// advance identically whether or not the degenerate case was hit — otherwise
/// the same seed produces divergent downstream draws the moment a degenerate
/// range is crossed, silently breaking the sim's seeded-replay determinism
/// pillar (GTW-644's defect was a guard that SKIPPED the degenerate draw).
pub(super) fn uniform_or_midpoint(rng: &mut impl Rng, range: Range<f32>) -> RangeSample {
    if range.start < range.end {
        // Live range: the plain uniform draw — byte-identical to `random_range`.
        return RangeSample::new(rng.random_range(range));
    }
    // Degenerate (empty or inverted): CONSUME the one draw anyway, then collapse
    // to the midpoint. The unit range is never empty, so this cannot panic.
    let _: f32 = rng.random_range(0.0_f32..1.0);
    RangeSample::new(f32::midpoint(range.start, range.end))
}

#[cfg(test)]
mod test {
    use rand::SeedableRng;
    use rand_chacha::ChaCha12Rng;

    use super::uniform_or_midpoint;

    /// An arbitrary fixed seed — determinism is the property, the value is irrelevant.
    const SEED: u64 = 0x0640_0644_5AFE_D4A3;

    /// A live range draws BYTE-IDENTICALLY to a plain `random_range` — the helper
    /// changes nothing for valid ranges (existing seeded replays are untouched).
    #[test]
    fn live_range_is_byte_identical_to_plain_random_range() {
        use rand::RngExt as _;
        let mut through_helper = ChaCha12Rng::seed_from_u64(SEED);
        let mut plain = ChaCha12Rng::seed_from_u64(SEED);
        for _ in 0..64 {
            let helper_draw = uniform_or_midpoint(&mut through_helper, 2.0..3.0);
            let plain_draw: f32 = plain.random_range(2.0..3.0);
            assert_eq!(
                helper_draw.to_bits(),
                plain_draw.to_bits(),
                "a live range must draw byte-identically to random_range",
            );
        }
    }

    /// A degenerate range (empty AND inverted) returns the bounds' midpoint — and
    /// the width-zero §7 band `[1 − v, 1 + v]` at `v = 0` collapses to factor 1.0.
    #[test]
    fn degenerate_range_returns_the_midpoint() {
        let mut rng = ChaCha12Rng::seed_from_u64(SEED);
        // Empty (start == end): the point itself.
        assert_eq!(
            uniform_or_midpoint(&mut rng, 1.0..1.0).to_bits(),
            1.0_f32.to_bits(),
            "an empty range collapses to its point",
        );
        // Inverted (start > end): the midpoint of the two bounds.
        assert_eq!(
            uniform_or_midpoint(&mut rng, 1.5..0.5).to_bits(),
            1.0_f32.to_bits(),
            "an inverted range collapses to the bounds' midpoint",
        );
    }

    /// The draw-count invariant: a degenerate call consumes EXACTLY as much stream
    /// as a live call — two same-seeded streams, one routed through a degenerate
    /// range and one through a live range, stay aligned on their subsequent draws.
    #[test]
    fn degenerate_call_consumes_exactly_one_draw() {
        use rand::RngExt as _;
        let mut through_degenerate = ChaCha12Rng::seed_from_u64(SEED);
        let mut through_live = ChaCha12Rng::seed_from_u64(SEED);

        let _ = uniform_or_midpoint(&mut through_degenerate, 1.0..1.0);
        let _ = uniform_or_midpoint(&mut through_live, 0.8..1.2);

        for i in 0..64 {
            let a: u64 = through_degenerate.random();
            let b: u64 = through_live.random();
            assert_eq!(
                a, b,
                "draw {i}: a degenerate call must advance the stream exactly one draw, \
                 keeping same-seeded streams aligned",
            );
        }
    }
}
