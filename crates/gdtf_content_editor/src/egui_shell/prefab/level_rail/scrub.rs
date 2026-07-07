//! The rail's **wheel-scrub accumulator** (GTW-595 C1) — folds fractional scroll points
//! into whole storey steps, keeping the sub-step remainder across frames so a smooth
//! trackpad scroll scrubs the levels at a steady, predictable rate.

/// How many scroll points move the selection one storey — the same order of feel as the
/// viewport's `SCROLL_PER_NOTCH` zoom (a wheel notch ≈ one step). A framework tuning
/// const (the wheel feel), not a domain value.
const POINTS_PER_STOREY: f32 = 50.0;

/// The carried sub-step scroll remainder (no-bare-types: the scrub position between two
/// storey steps is a domain value). Private inner; folded through
/// [`fold`](ScrubAccumulator::fold) only.
#[derive(Default, Debug)]
pub(crate) struct ScrubAccumulator(f32);

impl ScrubAccumulator {
    /// Fold `points` of scroll into the accumulator and return the WHOLE storey steps it
    /// yields (positive = up, negative = down), keeping the fractional remainder for the
    /// next frame. Multipass-safe: egui takes the frame input after the first pass, so a
    /// re-run folds `0.0` and yields no extra step (bevy-traps #8 fact (b)).
    pub(super) fn fold(&mut self, points: f32) -> i32 {
        self.0 += points / POINTS_PER_STOREY;
        // A storey rail is at most MAX_LEVELS (8) tall — clamp before the cast so even a
        // pathological delta stays a sane, representable step count.
        let steps = self.0.trunc().clamp(-64.0, 64.0);
        self.0 -= steps;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "steps is trunc()'d to a whole number and clamped into [-64, 64], so the \
                      i32 cast is exact"
        )]
        let whole = steps as i32;
        whole
    }
}
