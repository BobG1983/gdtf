//! The stat-block widget colors (GTW-278) — mockup-approximate fills for the TU / HP
//! bars and the Wounds pips.
//!
//! Per the ticket these are deliberately simple, mockup-approximate constants ("whatever's
//! easiest"), NOT a tuned palette: the `gdtf_ui` `ProgressBar` / `Pips` widgets take any
//! two [`Color`]s, and the stat block pins the mockup pairing (HP green/red, TU
//! accent/dark, Wounds yellow/dark) here in one place. Theme-derived re-coloring of these
//! is a later polish pass; these are plain `Color` plumbing fed straight to the widgets
//! (the `CELL_PX`-class framework carve-out, `.claude/rules/no-bare-types.md` clause 4).

use bevy::color::Color;

/// The TU bar's **remaining** (filled) color — a cyan-ish accent.
pub(in crate::scenes::running::game::battlescape) const TU_REMAINING: Color =
    Color::srgb(0.36, 0.78, 0.92);

/// The TU bar's **lost** (track) color — a dark panel-bg.
pub(in crate::scenes::running::game::battlescape) const TU_LOST: Color =
    Color::srgb(0.12, 0.14, 0.16);

/// The HP bar's **remaining** (filled) color — green.
pub(in crate::scenes::running::game::battlescape) const HP_REMAINING: Color =
    Color::srgb(0.30, 0.78, 0.36);

/// The HP bar's **lost** (track) color — red.
pub(in crate::scenes::running::game::battlescape) const HP_LOST: Color =
    Color::srgb(0.70, 0.18, 0.18);

/// The Wounds pips' **remaining** (filled) color — yellow.
pub(in crate::scenes::running::game::battlescape) const WOUNDS_REMAINING: Color =
    Color::srgb(0.92, 0.82, 0.24);

/// The Wounds pips' **lost** (empty) color — a dark panel-bg.
pub(in crate::scenes::running::game::battlescape) const WOUNDS_LOST: Color =
    Color::srgb(0.12, 0.14, 0.16);
