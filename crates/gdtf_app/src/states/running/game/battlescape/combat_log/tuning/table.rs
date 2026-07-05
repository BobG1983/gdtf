//! The resolved [`CombatLogTuning`] table and its hot-RON registration onto the GTW-564
//! generic seam. Split out of the monolithic `tuning.rs` (GTW-583); the tuning
//! rationale lives on the parent `tuning` module.

use bevy::prelude::*;
use gdtf_assets::HotRonAppExt;
use serde::Deserialize;

use super::{
    BottomClearanceLines, FadeFraction, FadeInSeconds, FadeOutSeconds, HeightLerpRate, LineFontPt,
    LineLerpRate, LineTtlSeconds, MaxVisibleLines, PanelWidthVw,
};

/// The HOT-RELOADABLE combat-log tuning table — the four feel numbers, loaded from
/// `assets/core_tuning/combat_log.tuning.ron` and read live by the combat-log spawn + update systems.
///
/// Loaded from the loose `assets/core_tuning/combat_log.tuning.ron` through the GTW-564
/// generic hot-RON chain ([`register_combat_log_hot_ron`]) and resolved into this
/// [`CombatLogTuning`] resource, then re-derived in place on a hot edit — the SAME
/// dual-role spec-IS-the-resolved-resource shape the presenter's
/// [`FxTuning`](gdtf_battle_presenter::FxTuning) uses (the value clones straight out of the
/// `RonAsset`, no extra resolve step).
///
/// Each field is `#[serde(default)]` so a `.ron` that omits a field falls back to the shipped
/// value rather than failing to parse — an author can tune one number without restating the
/// rest.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored `.ron`
/// shape), [`Default`] (the all-shipped-values fallback the headless app and a partial `.ron`
/// use), and [`TypePath`] (the bound [`RonAsset<CombatLogTuning>`](gdtf_assets::RonAsset)
/// requires of its payload).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Deserialize, TypePath, Default)]
#[serde(default)]
pub(crate) struct CombatLogTuning {
    /// How many log lines stay visible before the oldest FIFO-despawns.
    pub(crate) max_visible_lines:      MaxVisibleLines,
    /// How long each log line lives before it fades + despawns (seconds).
    pub(crate) line_ttl_seconds:       LineTtlSeconds,
    /// The fraction of a line's lifetime spent fading at the end.
    pub(crate) fade_fraction:          FadeFraction,
    /// The log panel's width as a fraction of the window width (vw).
    pub(crate) panel_width_vw:         PanelWidthVw,
    /// The point size each log line is drawn at (the log's readable body size, larger than theme).
    pub(crate) line_font_pt:           LineFontPt,
    /// How fast a line eases toward its target slot as the stack reflows (per-second LERP rate).
    pub(crate) line_lerp_rate:         LineLerpRate,
    /// How long a line spends fading IN on appear (seconds).
    pub(crate) fade_in_seconds:        FadeInSeconds,
    /// How long a line spends fading OUT at end of life (seconds).
    pub(crate) fade_out_seconds:       FadeOutSeconds,
    /// How fast the panel height eases toward its natural content height (per-second LERP rate).
    pub(crate) height_lerp_rate:       HeightLerpRate,
    /// The bottom clearance below the newest line (line-height multiple) so the clip never shaves
    /// the bottommost line's descenders at the panel / bottom-bar boundary (the cut-off fix).
    pub(crate) bottom_clearance_lines: BottomClearanceLines,
}

/// The path of the loose combat-log RON, relative to the asset source root.
const COMBAT_LOG_RON_PATH: &str = "core_tuning/combat_log.tuning.ron";

/// Registers the [`CombatLogTuning`] hot-RON chain — ONE ext call onto the
/// GTW-564 generic seam (kick-off / gated resolve / live redrive, keyed by the
/// generic [`HotRonHandle`](gdtf_assets::HotRonHandle)`<CombatLogTuning>`),
/// replacing the per-site handle newtype + load/resolve/redrive triple.
/// Self-gates on the [`AssetServer`](bevy::asset::AssetServer)
/// (`bevy-traps.md` #1), so a `MinimalPlugins` headless app stays a no-op (the
/// log then runs on [`CombatLogTuning::default`]). On a live `.ron` edit the
/// generic redrive overwrites [`CombatLogTuning`] through `ResMut`, so a
/// `combat_log.tuning.ron` edit re-tunes the log THIS frame — WITHOUT a rebuild.
pub(crate) fn register_combat_log_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<CombatLogTuning>(COMBAT_LOG_RON_PATH);
}
