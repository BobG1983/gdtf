use bevy::prelude::*;
use cobalt_ron_assets::HotRonAppExt;
use serde::Deserialize;

use super::{
    BottomClearanceLines, FadeFraction, FadeInSeconds, FadeOutSeconds, HeightLerpRate, LineFontPt,
    LineLerpRate, LineTtlSeconds, MaxVisibleLines, PanelWidthVw,
};

/// Each field is `#[serde(default)]` so a `.ron` that omits a field falls back to the shipped
#[derive(Resource, Debug, Clone, Copy, PartialEq, Deserialize, TypePath, Default)]
#[serde(default)]
pub(crate) struct CombatLogTuning {
    pub(crate) max_visible_lines:      MaxVisibleLines,
    pub(crate) line_ttl_seconds:       LineTtlSeconds,
    pub(crate) fade_fraction:          FadeFraction,
    pub(crate) panel_width_vw:         PanelWidthVw,
    pub(crate) line_font_pt:           LineFontPt,
    pub(crate) line_lerp_rate:         LineLerpRate,
    pub(crate) fade_in_seconds:        FadeInSeconds,
    pub(crate) fade_out_seconds:       FadeOutSeconds,
    pub(crate) height_lerp_rate:       HeightLerpRate,
    pub(crate) bottom_clearance_lines: BottomClearanceLines,
}

const COMBAT_LOG_RON_PATH: &str = "core_tuning/combat_log.tuning.ron";

pub(crate) fn register_combat_log_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<CombatLogTuning>(COMBAT_LOG_RON_PATH);
}
