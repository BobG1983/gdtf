//! Unit tests for the combat-log anim components: the `LogLineFade` three-phase curve,
//! the `LineSlide` reflow ease, and the `PanelHeightAnim` height ease (GTW-328 slice B).

use std::time::Duration;

use super::{LineSlide, LogLineFade, PanelHeightAnim};
use crate::states::running::game::battlescape::combat_log::tuning::{
    FadeInSeconds, FadeOutSeconds, LineTtlSeconds,
};

/// A line that is `f32::EPSILON`-close to `expected` (the float-cmp idiom; clippy `float_cmp`
/// denies `==` on `f32`). A small slack so the `clamp`/`mul_add` rounding does not flake.
fn approx(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < 1e-4
}

/// Build the three-phase fade for a `ttl`-second line with the given fade-in / fade-out
/// windows, scaling from full opacity (`1.0`).
fn fade(ttl: f32, fade_in: f32, fade_out: f32) -> LogLineFade {
    LogLineFade::new(
        LineTtlSeconds::from_secs(ttl),
        FadeInSeconds::from_secs(fade_in),
        FadeOutSeconds::from_secs(fade_out),
        1.0,
    )
}

// ---- LogLineFade — the three-phase fade-in / hold / fade-out curve (GTW-328 slice B). ----

/// On spawn (zero elapsed) a line's alpha is `0` — it FADES IN rather than snapping to full
/// opacity.
#[test]
fn a_fresh_line_starts_transparent_and_fades_in() {
    let f = fade(5.0, 0.2, 1.0);
    assert!(
        approx(f.alpha(), 0.0),
        "alpha at spawn is 0, got {}",
        f.alpha()
    );
}

/// Partway through the fade-IN window the alpha is a partial ramp toward full (here halfway
/// through a `0.2`s fade-in => ~`0.5`).
#[test]
fn mid_fade_in_the_alpha_is_a_partial_ramp_up() {
    let mut f = fade(5.0, 0.2, 1.0);
    f.advance(Duration::from_millis(100)); // halfway through the 0.2s fade-in
    assert!(
        approx(f.alpha(), 0.5),
        "halfway through fade-in alpha is ~0.5, got {}",
        f.alpha(),
    );
}

/// After the fade-in window and before the fade-out window the line HOLDS at full opacity.
#[test]
fn the_hold_phase_is_full_opacity() {
    let mut f = fade(5.0, 0.2, 1.0);
    f.advance(Duration::from_secs_f32(2.0)); // well past fade-in (0.2), well before fade-out
    assert!(
        approx(f.alpha(), 1.0),
        "in the hold phase alpha is full, got {}",
        f.alpha(),
    );
}

/// Inside the fade-OUT window (the final `fade_out` seconds) the alpha ramps from full toward
/// `0` (here halfway through the `1.0`s fade-out => ~`0.5`).
#[test]
fn mid_fade_out_the_alpha_is_a_partial_ramp_down() {
    let mut f = fade(5.0, 0.2, 1.0);
    // Fade-out starts at life - fade_out = 4.0s; advance to 4.5s (halfway through the 1.0s tail).
    f.advance(Duration::from_secs_f32(4.5));
    assert!(
        approx(f.alpha(), 0.5),
        "halfway through fade-out alpha is ~0.5, got {}",
        f.alpha(),
    );
}

/// Overlapping fade windows (fade-in + fade-out exceed the life) are scaled to FIT — neither
/// is dropped, and the curve still completes (the fade-out reaches ~0 by end of life).
#[test]
fn overlapping_fade_windows_are_scaled_to_fit_the_life() {
    // 1s life, but 0.8 + 0.8 = 1.6s of fades requested -> scaled to 0.5 + 0.5.
    let mut f = fade(1.0, 0.8, 0.8);
    f.advance(Duration::from_secs_f32(0.999)); // essentially end of life
    assert!(
        f.alpha() < 0.1,
        "with windows scaled to fit, the alpha still fades to ~0 by end of life, got {}",
        f.alpha(),
    );
}

/// The clock finishes (signals despawn) the frame its TTL elapses.
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

// ---- LineSlide — the per-line position ease toward the target slot (GTW-328 slice B). ----

/// A fresh slide carries its seeded appear-offset, and easing toward the target (`0`) STRICTLY
/// SHRINKS it (never snaps) until it settles at exactly `0`.
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

    // Many steps settle it to exactly 0 (the sub-pixel snap).
    for _ in 0..200 {
        slide.ease_toward_target(0.3);
    }
    assert!(
        approx(slide.current(), 0.0),
        "the slide settles at exactly 0, got {}",
        slide.current(),
    );
}

/// A reflow DISPLACES the offset by the freed height — the survivor holds its old screen
/// position (a bigger offset) before easing back into the gap.
#[test]
fn a_reflow_displaces_the_slide_by_the_freed_height() {
    let mut slide = LineSlide::new(0.0); // already settled
    slide.displace(15.0); // a line above was removed; bump by its height
    assert!(
        approx(slide.current(), 15.0),
        "the freed height is added to the offset, got {}",
        slide.current(),
    );
}

// ---- PanelHeightAnim — the root height ease toward the content height (GTW-328 slice B). ----

/// The panel height GROWS toward a taller target as lines are added (eased, not snapped) and
/// settles at exactly the target.
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

/// The panel height SHRINKS toward a shorter target as lines are removed (eased, not snapped).
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
