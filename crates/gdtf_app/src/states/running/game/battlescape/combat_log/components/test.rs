use std::time::Duration;

use super::{LineSlide, LogLineFade, PanelHeightAnim};
use crate::states::running::game::battlescape::combat_log::tuning::{
    FadeInSeconds, FadeOutSeconds, LineTtlSeconds,
};

fn approx(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < 1e-4
}

fn fade(ttl: f32, fade_in: f32, fade_out: f32) -> LogLineFade {
    LogLineFade::new(
        LineTtlSeconds::from_secs(ttl),
        FadeInSeconds::from_secs(fade_in),
        FadeOutSeconds::from_secs(fade_out),
        1.0,
    )
}

#[test]
fn a_fresh_line_starts_transparent_and_fades_in() {
    let f = fade(5.0, 0.2, 1.0);
    assert!(
        approx(f.alpha(), 0.0),
        "alpha at spawn is 0, got {}",
        f.alpha()
    );
}

#[test]
fn mid_fade_in_the_alpha_is_a_partial_ramp_up() {
    let mut f = fade(5.0, 0.2, 1.0);
    f.advance(Duration::from_millis(100));
    assert!(
        approx(f.alpha(), 0.5),
        "halfway through fade-in alpha is ~0.5, got {}",
        f.alpha(),
    );
}

#[test]
fn the_hold_phase_is_full_opacity() {
    let mut f = fade(5.0, 0.2, 1.0);
    f.advance(Duration::from_secs_f32(2.0));
    assert!(
        approx(f.alpha(), 1.0),
        "in the hold phase alpha is full, got {}",
        f.alpha(),
    );
}

#[test]
fn mid_fade_out_the_alpha_is_a_partial_ramp_down() {
    let mut f = fade(5.0, 0.2, 1.0);
    f.advance(Duration::from_secs_f32(4.5));
    assert!(
        approx(f.alpha(), 0.5),
        "halfway through fade-out alpha is ~0.5, got {}",
        f.alpha(),
    );
}

#[test]
fn overlapping_fade_windows_are_scaled_to_fit_the_life() {
    let mut f = fade(1.0, 0.8, 0.8);
    f.advance(Duration::from_secs_f32(0.999));
    assert!(
        f.alpha() < 0.1,
        "with windows scaled to fit, the alpha still fades to ~0 by end of life, got {}",
        f.alpha(),
    );
}

#[test]
fn the_clock_finishes_at_end_of_life() {
    let mut f = fade(0.5, 0.1, 0.1);
    assert!(
        !f.advance(Duration::from_millis(250)),
        "not finished mid-life"
    );
    assert!(
        f.advance(Duration::from_millis(300)),
        "the clock finishes once the TTL has elapsed",
    );
}

#[test]
fn a_slide_eases_toward_zero_without_snapping() {
    let mut slide = LineSlide::new(20.0);
    assert!(
        approx(slide.current(), 20.0),
        "the seeded offset is preserved"
    );

    let after_one = {
        slide.ease_toward_target(0.3);
        slide.current()
    };
    assert!(
        after_one < 20.0 && after_one > 0.0,
        "one ease step shrinks the offset toward 0 without snapping, got {after_one}",
    );

    for _ in 0..200 {
        slide.ease_toward_target(0.3);
    }
    assert!(
        approx(slide.current(), 0.0),
        "the slide settles at exactly 0, got {}",
        slide.current(),
    );
}

#[test]
fn a_reflow_displaces_the_slide_by_the_freed_height() {
    let mut slide = LineSlide::new(0.0);
    slide.displace(15.0);
    assert!(
        approx(slide.current(), 15.0),
        "the freed height is added to the offset, got {}",
        slide.current(),
    );
}

#[test]
fn the_panel_height_grows_toward_a_taller_target() {
    let mut anim = PanelHeightAnim::new(0.0);
    anim.ease_toward(100.0, 0.25);
    let after_one = anim.current();
    assert!(
        after_one > 0.0 && after_one < 100.0,
        "one ease step grows toward the target without snapping, got {after_one}",
    );
    for _ in 0..200 {
        anim.ease_toward(100.0, 0.25);
    }
    assert!(
        approx(anim.current(), 100.0),
        "the height settles at exactly the target, got {}",
        anim.current(),
    );
}

#[test]
fn the_panel_height_shrinks_toward_a_shorter_target() {
    let mut anim = PanelHeightAnim::new(100.0);
    anim.ease_toward(40.0, 0.25);
    let after_one = anim.current();
    assert!(
        after_one < 100.0 && after_one > 40.0,
        "one ease step shrinks toward the shorter target without snapping, got {after_one}",
    );
    for _ in 0..200 {
        anim.ease_toward(40.0, 0.25);
    }
    assert!(
        approx(anim.current(), 40.0),
        "the height settles at exactly the shorter target, got {}",
        anim.current(),
    );
}
