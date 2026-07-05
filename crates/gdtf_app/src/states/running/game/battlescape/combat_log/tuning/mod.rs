//! The HOT-RELOADABLE combat-log tuning table and its RON load / resolve / re-derive chain
//! (GTW-328, slice 3).
//!
//! The combat log's FEEL — how many lines stay on screen, how long each lives, how much of
//! that life is spent fading, and how wide the panel is — lives in a loose, per-line commented
//! `assets/core_tuning/combat_log.tuning.ron`, loaded through the SAME generic
//! [`RonAsset<T>`](gdtf_assets::RonAsset) loader the theme / FX-tuning / pan-tuning tables use,
//! so the user tunes the log WITHOUT a rebuild.
//!
//! Each value is a NAMED newtype (`.claude/rules/no-bare-types.md`) with a PRIVATE inner read
//! through [`Deref`](std::ops::Deref) and a [`Default`] carrying the shipped value, so a missing `.ron` field
//! degrades to the default rather than a parse error. The four resolve into one
//! [`CombatLogTuning`] resource the spawn + update systems READ (never a `const`), re-derived
//! in place on a hot edit through the GTW-564 generic hot-RON seam
//! ([`register_combat_log_hot_ron`]) — mirroring the presenter's
//! [`FxTuning`](gdtf_battle_presenter::FxTuning) hot-reload and the UI theme's.

mod lines;
mod panel;
mod table;

pub(crate) use lines::{
    FadeFraction, FadeInSeconds, FadeOutSeconds, LineFontPt, LineLerpRate, LineTtlSeconds,
    MaxVisibleLines,
};
pub(crate) use panel::{BottomClearanceLines, HeightLerpRate, PanelWidthVw};
pub(crate) use table::{CombatLogTuning, register_combat_log_hot_ron};

#[cfg(test)]
mod test;
