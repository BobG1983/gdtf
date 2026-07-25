//! The Options screen's layout constants — spacing and toggle/readout sizing.
//!
//! Kept out of the tree-building code so a calibration tweak is a one-file change, and so
//! every setting row (the sound row and, under `dev_tools`, the procgen-stepper row) is
//! sized from the SAME numbers rather than a copied literal.

use bevy::prelude::*;

/// The Options column's inter-child gap, in viewport-height percent (`Vh`).
///
/// A named newtype over the gap rather than a bare `f32` (no-bare-types rule): it
/// is layout spacing set on the flex column [`Node`] directly (the theme owns
/// palette + font, not inter-child layout), mirroring the menu's `ColumnGapVh`.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub(super) struct OptionsGapVh(f32);

impl OptionsGapVh {
    /// The screen's column gap, matching the menu's 10 px / 720 reference gap.
    pub(super) const SCREEN: Self = Self(1.38889);
}

/// The toggle pill's long (slide) dimension, in viewport-width units.
/// Calibrated to the hand-rolled switch's 48 px / 1280 pill so a toggle keeps the
/// same footprint beside its caption.
pub(super) const TOGGLE_LONG_VW: f32 = 3.75;

/// The toggle pill's short dimension, in viewport-width units (20 px / 1280) —
/// strictly taller than the knob so the pill frames it.
pub(super) const TOGGLE_SHORT_VW: f32 = 1.562_5;

/// The inner padding around the knob inside the toggle pill, in viewport-width units
/// (3 px / 1280) — keeps the knob clear of the rounded caps.
pub(super) const TOGGLE_PAD_VW: f32 = 0.234_375;

/// The knob diameter, in viewport-width units (14 px / 1280), smaller than the pill's
/// short dimension so the track frames it.
pub(super) const KNOB_DIAMETER_VW: f32 = 1.093_75;

/// A setting readout's fixed minimum width, in viewport-width units (32 px /
/// 1280) — sized to comfortably clear the WIDER of the two readout strings ("Off",
/// 3 chars at the 18 pt body font vs "On"'s 2). Fixing a floor on the label's width
/// means flipping a toggle never changes the label's intrinsic width, so the
/// auto-width setting row / panel / centered screen no longer reflows on every toggle
/// (GTW-800a).
pub(super) const SETTING_VALUE_MIN_WIDTH_VW: f32 = 2.5;
