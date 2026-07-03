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
//! through [`Deref`] and a [`Default`] carrying the shipped value, so a missing `.ron` field
//! degrades to the default rather than a parse error. The four resolve into one
//! [`CombatLogTuning`] resource the spawn + update systems READ (never a `const`), re-derived
//! in place on a hot edit through the GTW-564 generic hot-RON seam
//! ([`register_combat_log_hot_ron`]) — mirroring the presenter's
//! [`FxTuning`](gdtf_battle_presenter::FxTuning) hot-reload and the UI theme's.

use bevy::prelude::*;
use gdtf_assets::HotRonAppExt;
use serde::Deserialize;

/// How many combat-log lines stay visible at once before the OLDEST is FIFO-despawned.
///
/// A new event appends a line; once the visible count exceeds this, the oldest line is
/// despawned so the strip scrolls off the top. The default `6` is a readable history depth.
///
/// A named newtype over the `usize` count (`.claude/rules/no-bare-types.md`): the inner is
/// PRIVATE, read through [`Deref`] and built through [`Default`] (or serde deserialization).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub(crate) struct MaxVisibleLines(usize);

impl MaxVisibleLines {
    /// The shipped default: `6` visible log lines.
    pub(crate) const DEFAULT: usize = 6;
}

impl Default for MaxVisibleLines {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// How long each combat-log line lives before it fades out + despawns, in SECONDS.
///
/// A line holds at full opacity for most of this window, then fades over the tail (see
/// [`FadeFraction`]) and despawns the frame its clock finishes. The default `5.0` s is a
/// readable lifetime (long enough to catch the event, short enough to keep the log fresh).
///
/// A named newtype over the `f32` seconds (`.claude/rules/no-bare-types.md`): the inner is
/// PRIVATE, read through [`Deref`] and built through [`Default`] (or serde deserialization). The live
/// per-line clock IS a separate component ([`super::components::LogLineFade`]); this is the
/// tunable lifetime it runs for.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct LineTtlSeconds(f32);

impl LineTtlSeconds {
    /// The shipped default: `5.0` s per log line.
    pub(crate) const DEFAULT: f32 = 5.0;

    /// Build a TTL of `secs` seconds — test-only, so the components' fade tests can drive a
    /// short, deterministic lifetime through the real [`LogLineFade`] constructor without the
    /// async RON load (the private inner stays sealed in shipped code).
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn from_secs(secs: f32) -> Self {
        Self(secs)
    }
}

impl Default for LineTtlSeconds {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// The fraction of a line's lifetime spent FADING at the end (`0.0..=1.0`).
///
/// A line stays fully opaque for `1 - this` of its life, then its alpha ramps from full to `0`
/// over the final `this`-fraction. The default `0.35` keeps the line readable for the first
/// ~65% of its TTL, fading across the last ~35%.
///
/// A named newtype over the `f32` fraction (`.claude/rules/no-bare-types.md`): the inner is
/// PRIVATE, read through [`Deref`] and built through [`Default`] (or serde deserialization).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct FadeFraction(f32);

impl FadeFraction {
    /// The shipped default: fade across the final `0.35` of the line's life.
    pub(crate) const DEFAULT: f32 = 0.35;
}

impl Default for FadeFraction {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// How fast a combat-log line eases toward its TARGET slot as the stack reflows — the per-second
/// LERP rate of the slide animation (GTW-328 slice B, animation polish).
///
/// When a new line appends below, or a faded line is removed above, every surviving line's
/// position offset is eased toward `0` (its natural flex slot) by `1 - exp(-rate * delta)` each
/// frame — never SNAPPED. Bigger = a snappier, faster settle; smaller = a longer, gentler glide.
/// The default `12.0`/s settles a typical reflow in roughly a quarter-second (a gentle,
/// readable slide).
///
/// A named newtype over the `f32` per-second rate (`.claude/rules/no-bare-types.md`): the inner
/// is PRIVATE, read through [`Deref`] and built through [`Default`] (or serde deserialization).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct LineLerpRate(f32);

impl LineLerpRate {
    /// The shipped default: ease toward the target slot at `12.0`/s (a gentle settle).
    pub(crate) const DEFAULT: f32 = 12.0;
}

impl Default for LineLerpRate {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// How long a combat-log line spends FADING IN on appear, in SECONDS (GTW-328 slice B).
///
/// A fresh line ramps its alpha from `0` to full over this window before its hold/fade-out
/// lifetime begins — it appears gently rather than snapping on at full opacity. Bigger = a
/// slower, softer entrance; `0.0` = an instant on. The default `0.18` s is a quick, readable
/// fade-in.
///
/// A named newtype over the `f32` seconds (`.claude/rules/no-bare-types.md`): the inner is
/// PRIVATE, read through [`Deref`] and built through [`Default`] (or serde deserialization).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct FadeInSeconds(f32);

impl FadeInSeconds {
    /// The shipped default: fade in over `0.18` s on appear.
    pub(crate) const DEFAULT: f32 = 0.18;

    /// Build a fade-in of `secs` seconds — test-only (see [`LineTtlSeconds::from_secs`]).
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn from_secs(secs: f32) -> Self {
        Self(secs)
    }
}

impl Default for FadeInSeconds {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// How long a combat-log line spends FADING OUT at end of life, in SECONDS (GTW-328 slice B).
///
/// After fade-in + hold, a line ramps its alpha from full back to `0` over this tail window,
/// then despawns the frame its lifetime clock finishes — it leaves gently rather than blinking
/// off. Bigger = a slower, softer exit; the default `0.9` s is a readable fade-out. This is the
/// absolute-duration sibling of [`FadeFraction`] (the fraction-of-TTL form); the live clock
/// derives the fade-out window from whichever the tuning provides.
///
/// A named newtype over the `f32` seconds (`.claude/rules/no-bare-types.md`): the inner is
/// PRIVATE, read through [`Deref`] and built through [`Default`] (or serde deserialization).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct FadeOutSeconds(f32);

impl FadeOutSeconds {
    /// The shipped default: fade out over `0.9` s at end of life.
    pub(crate) const DEFAULT: f32 = 0.9;

    /// Build a fade-out of `secs` seconds — test-only (see [`LineTtlSeconds::from_secs`]).
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn from_secs(secs: f32) -> Self {
        Self(secs)
    }
}

impl Default for FadeOutSeconds {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

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

/// The point size each combat-log LINE is drawn at — the log's own readable body size, kept
/// DELIBERATELY LARGER than the theme's body text so the strip reads cleanly over the HUD.
///
/// Each log line takes its font size from this value (not the theme's body `font_size_pt`, which is
/// the panel-label size and rendered the log too small). A lethal DOWN/DEAD line is still drawn at
/// a heavier WEIGHT (and a small size bump) on top of this — see the spawn system's emphasis
/// scaling. The default `20.0` pt is clearly larger than the theme's `18.0` pt body text, so a line
/// like `"Alex Mercer moved (15, 11) -> (14, 12)"` is legible at a glance. RON-tunable, so the user
/// can dial it without a rebuild. Font size in pt is the sanctioned `ui-responsive-not-px` carve-out
/// (`.claude/rules/ui-responsive-not-px.md` — a glyph size is intrinsically a point measure).
///
/// A named newtype over the `f32` pt (`.claude/rules/no-bare-types.md`): the inner is PRIVATE,
/// read through [`Deref`] and built through [`Default`] (or serde deserialization).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub(crate) struct LineFontPt(f32);

impl LineFontPt {
    /// The shipped default: `20.0` pt — clearly larger than the theme's `18.0` pt body text so the
    /// log reads well over the HUD.
    pub(crate) const DEFAULT: f32 = 20.0;
}

impl Default for LineFontPt {
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
/// the animated-height target ([`animate_combat_log_height`](super::systems::animate_combat_log_height)):
/// the panel grows by this much beyond the content sum, and since the column is top-aligned the extra
/// space lands at the BOTTOM — lifting the bottommost line's full glyph box ABOVE the clip edge. It is
/// applied to the height target rather than as `Node::padding` because the panel is a themed
/// [`spawn_panel`](gdtf_ui::spawn_panel) box whose `padding` is re-derived from the theme by
/// `apply_theme` every repaint (it would clobber a spawn-time padding); the height target is the
/// panel's own animated lever, untouched by the theme pass.
///
/// The default `0.5` reserves about half a line of clearance below the bottommost line. Expressed as
/// a multiple of the line height (which scales with the responsive [`LineFontPt`] glyph size) rather
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

#[cfg(test)]
mod test {
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
            include_str!("../../../../../../../../assets/core_tuning/combat_log.tuning.ron");
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
        assert!(
            (*tuning.bottom_clearance_lines - BottomClearanceLines::DEFAULT).abs() < f32::EPSILON
        );
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
}
