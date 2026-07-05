//! The per-LINE feel newtypes of the combat-log tuning: visible count, lifetime, the
//! fade windows, the slide rate, and the line font size. Split out of the monolithic
//! `tuning.rs` (GTW-583); the tuning rationale lives on the parent `tuning` module.

use bevy::prelude::*;
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
/// per-line clock IS a separate component ([`super::super::components::LogLineFade`]); this is the
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
