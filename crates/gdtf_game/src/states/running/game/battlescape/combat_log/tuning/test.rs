use super::{
    BottomClearanceLines, CombatLogTuning, FadeFraction, FadeInSeconds, FadeOutSeconds,
    HeightLerpRate, LineFontPt, LineLerpRate, LineTtlSeconds, MaxVisibleLines, PanelWidthVw,
};

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

#[test]
fn the_defaults_are_the_documented_values() {
    let tuning = CombatLogTuning::default();
    assert_eq!(*tuning.max_visible_lines, MaxVisibleLines::DEFAULT);
    assert!((*tuning.line_ttl_seconds - LineTtlSeconds::DEFAULT).abs() < f32::EPSILON);
    assert!((*tuning.fade_fraction - FadeFraction::DEFAULT).abs() < f32::EPSILON);
    assert!((*tuning.panel_width_vw - PanelWidthVw::DEFAULT).abs() < f32::EPSILON);
    assert!((*tuning.line_font_pt - LineFontPt::DEFAULT).abs() < f32::EPSILON);
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
    const {
        assert!(
            BottomClearanceLines::DEFAULT > 0.0,
            "the bottom clearance must be positive so the bottommost line is not clipped",
        );
    }
}
