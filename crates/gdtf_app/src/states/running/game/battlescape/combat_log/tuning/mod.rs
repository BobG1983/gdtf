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
