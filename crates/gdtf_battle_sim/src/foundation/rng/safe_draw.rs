use core::ops::Range;

use bevy::prelude::Deref;
use rand::{Rng, RngExt};

#[derive(Deref, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub(super) struct RangeSample(f32);

impl RangeSample {
        #[must_use]
    const fn new(value: f32) -> Self {
        Self(value)
    }
}

pub(super) fn uniform_or_midpoint(rng: &mut impl Rng, range: Range<f32>) -> RangeSample {
    if range.start < range.end {
        return RangeSample::new(rng.random_range(range));
    }
    let _: f32 = rng.random_range(0.0_f32..1.0);
    RangeSample::new(f32::midpoint(range.start, range.end))
}

#[cfg(test)]
mod test {
    use rand::SeedableRng;
    use rand_chacha::ChaCha12Rng;

    use super::uniform_or_midpoint;

        const SEED: u64 = 0x0640_0644_5AFE_D4A3;

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
                "a live range must draw identically to random_range",
            );
        }
    }

            #[test]
    fn degenerate_range_returns_the_midpoint() {
        let mut rng = ChaCha12Rng::seed_from_u64(SEED);
        assert_eq!(
            uniform_or_midpoint(&mut rng, 1.0..1.0).to_bits(),
            1.0_f32.to_bits(),
            "an empty range collapses to its point",
        );
        assert_eq!(
            uniform_or_midpoint(&mut rng, 1.5..0.5).to_bits(),
            1.0_f32.to_bits(),
            "an inverted range collapses to the bounds' midpoint",
        );
    }

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
