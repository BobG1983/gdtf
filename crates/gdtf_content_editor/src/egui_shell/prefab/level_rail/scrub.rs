const POINTS_PER_STOREY: f32 = 50.0;

#[derive(Default, Debug)]
pub(crate) struct ScrubAccumulator(f32);

impl ScrubAccumulator {
    pub(super) fn fold(&mut self, points: f32) -> i32 {
        self.0 += points / POINTS_PER_STOREY;
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
