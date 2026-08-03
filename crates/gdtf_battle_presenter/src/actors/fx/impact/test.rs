use std::time::Duration;

use gdtf_battle_sim::weapon::DamageType;

use super::animation::{IMPACT_FRAME_SCALES, ImpactAnimation, ImpactStep, impact_frame_scale};
use crate::fx::{roles::IMPACT_FRAME_COUNT, tuning::ImpactFrameSeconds};

const FRAME_SECONDS: f32 = ImpactFrameSeconds::DEFAULT;

#[test]
fn advance_steps_through_all_frames_then_finishes() {
    assert_eq!(
        IMPACT_FRAME_COUNT, 3,
        "the impact animation must be a 3-frame sequence",
    );

    let mut anim = ImpactAnimation::new(DamageType::Kinetic, ImpactFrameSeconds::default());
    assert_eq!(
        anim.damage(),
        DamageType::Kinetic,
        "the animation carries the shot's damage type (selects the strip)",
    );

    let half = Duration::from_secs_f32(FRAME_SECONDS / 2.0);
    assert_eq!(
        anim.advance(half),
        ImpactStep::Showing(0),
        "a partial tick holds the current frame (frame 0)",
    );

    let full = Duration::from_secs_f32(FRAME_SECONDS + 0.001);
    for expected in 1..IMPACT_FRAME_COUNT {
        assert_eq!(
            anim.advance(full),
            ImpactStep::Showing(expected),
            "the {expected}-th window must step to frame {expected}",
        );
    }

    assert_eq!(
        anim.advance(full),
        ImpactStep::Finished,
        "the last frame's window elapsing must report Finished (despawn)",
    );
}

#[test]
fn impact_frame_scale_grows_then_clamps_to_the_last_frame() {
    assert_eq!(
        IMPACT_FRAME_SCALES.len(),
        IMPACT_FRAME_COUNT,
        "the impact scale table must carry one entry per impact frame",
    );
    for window in IMPACT_FRAME_SCALES.windows(2) {
        if let [smaller, larger] = window {
            assert!(
                larger > smaller,
                "each impact frame must draw LARGER than the prior (an expanding ring): \
                 {larger} must exceed {smaller}",
            );
        }
    }
    for (frame, expected) in IMPACT_FRAME_SCALES.iter().enumerate() {
        assert!(
            (impact_frame_scale(frame) - *expected).abs() < f32::EPSILON,
            "frame {frame} must draw at its table scale {expected}, \
             got {}",
            impact_frame_scale(frame),
        );
    }
    let last = IMPACT_FRAME_SCALES.last().copied().unwrap_or(1.0);
    assert!(
        (impact_frame_scale(IMPACT_FRAME_COUNT + 5) - last).abs() < f32::EPSILON,
        "an out-of-range impact frame must clamp to the last (largest) scale {last}, \
         got {}",
        impact_frame_scale(IMPACT_FRAME_COUNT + 5),
    );
}
