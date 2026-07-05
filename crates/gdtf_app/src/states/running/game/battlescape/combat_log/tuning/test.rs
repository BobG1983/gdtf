//! Unit tests for the combat-log tuning: the shipped-RON parse round-trip and the
//! documented-default invariants.

use super::{
    BottomClearanceLines, CombatLogTuning, FadeFraction, FadeInSeconds, FadeOutSeconds,
    HeightLerpRate, LineFontPt, LineLerpRate, LineTtlSeconds, MaxVisibleLines, PanelWidthVw,
};

/// The shipped `combat_log.tuning.ron` parses into `CombatLogTuning` and carries every tuning
/// value — a `ron::de` round-trip of the SHIPPED bytes (a missing-but-required field would
/// be a deserialize error; an absent field falls back to its `Default`).
#[test]
fn shipped_combat_log_ron_parses() {
    const SHIPPED: &str =
        include_str!("../../../../../../../../../assets/core_tuning/combat_log.tuning.ron");
    let parsed: Result<CombatLogTuning, _> = ron::de::from_str(SHIPPED);
    assert!(
        parsed.is_ok(),
        "shipped combat_log.tuning.ron must parse into CombatLogTuning, got: {:?}",
        parsed.as_ref().err(),
    );
}

/// The defaults match the documented shipped values, so an absent `.ron` (the headless
/// app) degrades to a sane log feel.
#[test]
fn the_defaults_are_the_documented_values() {
    let tuning = CombatLogTuning::default();
    assert_eq!(*tuning.max_visible_lines, MaxVisibleLines::DEFAULT);
    // Float fields: tolerance compare (clippy `float_cmp` denies `==` on `f32`).
    assert!((*tuning.line_ttl_seconds - LineTtlSeconds::DEFAULT).abs() < f32::EPSILON);
    assert!((*tuning.fade_fraction - FadeFraction::DEFAULT).abs() < f32::EPSILON);
    assert!((*tuning.panel_width_vw - PanelWidthVw::DEFAULT).abs() < f32::EPSILON);
    assert!((*tuning.line_font_pt - LineFontPt::DEFAULT).abs() < f32::EPSILON);
    // The log line size must read CLEARLY LARGER than the theme's body text (the bug fix):
    // a body-sized log was too small. 20pt default vs the theme's 18pt body — a compile-time
    // invariant (the `const` block keeps clippy's assertions-on-constants happy).
    const {
        assert!(
            LineFontPt::DEFAULT > 18.0,
            "the log line size must be clearly larger than the 18pt theme body text",
        );
    }
    assert!((*tuning.line_lerp_rate - LineLerpRate::DEFAULT).abs() < f32::EPSILON);
    assert!((*tuning.fade_in_seconds - FadeInSeconds::DEFAULT).abs() < f32::EPSILON);
    assert!((*tuning.fade_out_seconds - FadeOutSeconds::DEFAULT).abs() < f32::EPSILON);
    assert!((*tuning.height_lerp_rate - HeightLerpRate::DEFAULT).abs() < f32::EPSILON);
    assert!((*tuning.bottom_clearance_lines - BottomClearanceLines::DEFAULT).abs() < f32::EPSILON);
    // The bottom clearance must be STRICTLY POSITIVE — a zero clearance would leave the
    // bottommost line flush at the clip / bottom-bar edge and shave its descenders (the bug);
    // a compile-time invariant (the `const` block keeps clippy's assertions-on-constants happy).
    const {
        assert!(
            BottomClearanceLines::DEFAULT > 0.0,
            "the bottom clearance must be positive so the bottommost line is not clipped",
        );
    }
}
