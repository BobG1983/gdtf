//! Unit tests of the impact animation-stepper surface (moved whole from the old
//! `impact.rs` inline `mod test`).

use std::time::Duration;

use gdtf_battle_sim::weapon::DamageType;

use super::animation::{IMPACT_FRAME_SCALES, ImpactAnimation, ImpactStep, impact_frame_scale};
use crate::fx::{roles::IMPACT_FRAME_COUNT, tuning::ImpactFrameSeconds};

/// The per-impact-frame hold the unit tests drive the stepper with — the shipped
/// hot-reloadable default (what the resident `FxTuning` carries with no `.ron` override).
const FRAME_SECONDS: f32 = ImpactFrameSeconds::DEFAULT;

/// `ImpactAnimation::advance` — the exact per-frame stepper `animate_impact`'s
/// pass-2 drives — holds each frame for its window, steps through all
/// `IMPACT_FRAME_COUNT` frames in order, and reports `Finished` only once the
/// LAST frame's hold elapses: the 3-FRAME-advance-then-DESPAWN contract.
///
/// Driven purely off the timer (no `App` / no atlas), so it deterministically
/// pins the lifecycle the system reads: every `Showing(n)` is the tile pass-2
/// redraws at frame `n`, and `Finished` is the despawn signal.
#[test]
fn advance_steps_through_all_frames_then_finishes() {
    // A 3-frame strip is the authored impact length — assert the contract's count.
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

    // A partial tick HOLDS frame 0 (no step yet) — the same tile stays on screen.
    let half = Duration::from_secs_f32(FRAME_SECONDS / 2.0);
    assert_eq!(
        anim.advance(half),
        ImpactStep::Showing(0),
        "a partial tick holds the current frame (frame 0)",
    );

    // Each full window steps to the NEXT frame, in order, for frames 1..N-1.
    let full = Duration::from_secs_f32(FRAME_SECONDS + 0.001);
    for expected in 1..IMPACT_FRAME_COUNT {
        assert_eq!(
            anim.advance(full),
            ImpactStep::Showing(expected),
            "the {expected}-th window must step to frame {expected}",
        );
    }

    // The LAST frame's window finishing ends the animation (the despawn signal) —
    // 3 frames shown (0,1,2), then despawn.
    assert_eq!(
        anim.advance(full),
        ImpactStep::Finished,
        "the last frame's window elapsing must report Finished (despawn)",
    );
}

/// `impact_frame_scale` makes the impact a VISIBLY-EXPANDING shockwave (GTW-306 V3): the
/// per-frame draw scale STRICTLY GROWS frame 0 → last (so the burst grows into a ring), is
/// one entry per `IMPACT_FRAME_COUNT` frame, and an out-of-range frame degrades to the
/// LARGEST (last) scale rather than snapping back to 1×. The scale is UNIFORM by
/// construction (a single multiplier on both `custom_size` axes), which is what proves the
/// impact is enlarged uniformly, never stretched along an axis.
#[test]
fn impact_frame_scale_grows_then_clamps_to_the_last_frame() {
    // One scale entry per authored impact frame.
    assert_eq!(
        IMPACT_FRAME_SCALES.len(),
        IMPACT_FRAME_COUNT,
        "the impact scale table must carry one entry per impact frame",
    );
    // The scale strictly grows frame-to-frame — the expanding-shockwave read.
    for window in IMPACT_FRAME_SCALES.windows(2) {
        if let [smaller, larger] = window {
            assert!(
                larger > smaller,
                "each impact frame must draw LARGER than the prior (an expanding ring): \
                 {larger} must exceed {smaller}",
            );
        }
    }
    // Every in-range frame reads its own table entry (epsilon compare — these are f32).
    for (frame, expected) in IMPACT_FRAME_SCALES.iter().enumerate() {
        assert!(
            (impact_frame_scale(frame) - *expected).abs() < f32::EPSILON,
            "frame {frame} must draw at its table scale {expected}, \
             got {}",
            impact_frame_scale(frame),
        );
    }
    // An out-of-range frame degrades to the LAST (largest) scale, never 1× snap-back.
    let last = IMPACT_FRAME_SCALES.last().copied().unwrap_or(1.0);
    assert!(
        (impact_frame_scale(IMPACT_FRAME_COUNT + 5) - last).abs() < f32::EPSILON,
        "an out-of-range impact frame must clamp to the last (largest) scale {last}, \
         got {}",
        impact_frame_scale(IMPACT_FRAME_COUNT + 5),
    );
}
