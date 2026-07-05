//! The PANEL-level newtypes of the combat-log tuning: the height ease rate, the bottom
//! clearance below the newest line, and the panel width. Split out of the monolithic
//! `tuning.rs` (GTW-583); the tuning rationale lives on the parent `tuning` module.

use bevy::prelude::*;
use serde::Deserialize;

/// How fast the combat-log panel's HEIGHT eases toward its natural content height — the
/// per-second LERP rate of the panel-height animation (GTW-328 slice B).
///
/// As lines are added the panel's animated height grows toward the summed height of its visible
/// lines; as lines fade out / are removed it shrinks back DOWN toward the shorter content — both
/// eased by `1 - exp(-rate * delta)` each frame, never set in one frame. Bigger = a snappier
/// grow/shrink; smaller = a slower, more deliberate breathe. The default `10.0`/s settles a
/// height change in roughly a third of a second (a gentle, readable reflow).
///
/// A named newtype over the `f32` per-second rate (`.claude/rules/no-bare-types.md`): the inner
/// is PRIVATE, read through [`Deref`] and built through [`Default`] (or serde deserialization).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct HeightLerpRate(f32);

impl HeightLerpRate {
    /// The shipped default: ease the panel height toward the content height at `10.0`/s.
    pub(crate) const DEFAULT: f32 = 10.0;
}

impl Default for HeightLerpRate {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// The bottom CLEARANCE reserved below the newest (bottommost) combat-log line, as a multiple of a
/// line's height — so the panel clip never shaves the bottommost line's descenders + bottom leading
/// where the panel's bottom edge sits flush against the bottom bar (the cut-off fix).
///
/// The combat-log panel is bottom-anchored just above the bottom bar with [`Overflow::clip()`], and
/// its flex column is top-aligned, so the NEWEST line sits at the bottom — right at the clip / bottom-
/// bar boundary. With the animated height equal to the exact content sum, that bottommost line's
/// glyph box rests flush at the clip edge and its descenders are shaved. This clearance is ADDED to
/// the animated-height target ([`animate_combat_log_height`](super::super::systems::animate_combat_log_height)):
/// the panel grows by this much beyond the content sum, and since the column is top-aligned the extra
/// space lands at the BOTTOM — lifting the bottommost line's full glyph box ABOVE the clip edge. It is
/// applied to the height target rather than as `Node::padding` because the panel is a themed
/// [`spawn_panel`](gdtf_ui::spawn_panel) box whose `padding` is re-derived from the theme by
/// `apply_theme` every repaint (it would clobber a spawn-time padding); the height target is the
/// panel's own animated lever, untouched by the theme pass.
///
/// The default `0.5` reserves about half a line of clearance below the bottommost line. Expressed as
/// a multiple of the line height (which scales with the responsive [`LineFontPt`](super::lines::LineFontPt) glyph size) rather
/// than a fixed px, so it tracks the font on resize — the `ui-responsive-not-px` carve-out (a glyph-
/// relative measure), the same form [`LineHeight`](bevy::text::LineHeight)'s line box uses.
///
/// A named newtype over the `f32` line-height multiple (`.claude/rules/no-bare-types.md`): the inner
/// is PRIVATE, read through [`Deref`] and built through [`Default`] (or serde deserialization).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct BottomClearanceLines(f32);

impl BottomClearanceLines {
    /// The shipped default: reserve `0.5` of a line height of clearance below the bottommost line.
    pub(crate) const DEFAULT: f32 = 0.5;
}

impl Default for BottomClearanceLines {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// The combat-log panel's width, as a fraction of the WINDOW WIDTH ([`Val::Vw`](bevy::ui::Val)).
///
/// The bottom-left strip spans this much of the screen horizontally; its height is intrinsic to
/// its line count (flex column), so width is the one tunable layout lever. The default `28.0`
/// (28vw) fits the log's lines without crowding the map. Responsive (`Vw`) — never a fixed px
/// (`.claude/rules/ui-responsive-not-px.md`).
///
/// A named newtype over the `f32` vw (`.claude/rules/no-bare-types.md`): the inner is PRIVATE,
/// read through [`Deref`] and built through [`Default`] (or serde deserialization).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct PanelWidthVw(f32);

impl PanelWidthVw {
    /// The shipped default: `28.0` vw (28% of the window width).
    pub(crate) const DEFAULT: f32 = 28.0;
}

impl Default for PanelWidthVw {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}
