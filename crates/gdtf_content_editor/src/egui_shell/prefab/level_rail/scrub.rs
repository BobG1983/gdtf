const POINTS_PER_STOREY: f32 = 50.0;

#[derive(Default, Debug)]
pub(crate) struct ScrubAccumulator(f32);

impl ScrubAccumulator {
    pub(super) fn fold(&mut self, points: f32) -> i32 {
        self.0 += points / POINTS_PER_STOREY;
        let steps = self.0.trunc().clamp(-64.0, 64.0);
        self.0 -= steps;
        let whole = steps as i32;
        whole
    }
}
