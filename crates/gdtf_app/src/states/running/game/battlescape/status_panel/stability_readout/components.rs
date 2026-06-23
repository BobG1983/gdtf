//! Markers + the named presentation value for the status-panel **stability readout**
//! (GTW-345).
//!
//! The stability readout is a sibling row UNDER the status-panel root (NOT inside the
//! shared [`stat_block`](super::super::super::stat_block), which the inspect panel reuses —
//! a hovered target is not necessarily a shooter): a "STAB" caption above a
//! [`ProgressBar`](gdtf_ui::spawn_progress_bar) whose fill is the selected shooter's
//! **steadiness** (a fuller bar = a steadier shot). The bar is found for the per-update
//! mutate by its [`StabilityBar`] marker; the [`Steadiness`] newtype is the named
//! presentation value the [`ConeMult`](gdtf_battle_sim::ConeMult) readout crosses into the
//! widget as (no bare `f32` reaches the bar — `.claude/rules/no-bare-types.md`).

use bevy::prelude::*;
use gdtf_battle_sim::ConeMult;
use gdtf_ui::FillFraction;

crate::support_item! {
    /// Marks the **stability** [`ProgressBar`](gdtf_ui::spawn_progress_bar) track of the
    /// status panel — the bar whose fill the per-update mutate sets to the selected
    /// shooter's [`Steadiness`].
    ///
    /// The update finds this one track by marker (the status panel is the SOLE owner of a
    /// stability readout, so there is exactly one), then mutates its
    /// [`ProgressBarFill`](gdtf_ui::ProgressBarFill) child width via
    /// [`set_progress_bar`](gdtf_ui::set_progress_bar) ([[ui-mutate-not-respawn]]). A unit
    /// marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StabilityBar;
}

/// The status panel's **steadiness** readout value — the named presentation quantity the
/// [`ConeMult`] steadiness read crosses into the [`ProgressBar`](gdtf_ui::spawn_progress_bar)
/// as, so no bare `f32` reaches the widget (`.claude/rules/no-bare-types.md`).
///
/// A widget-presentation value (a normalized `0.0..=1.0` steadiness, fuller = steadier),
/// derived from the §1a [`ConeMult`] — the cone-WIDENING multiplier, where LOWER = steadier
/// = a NARROWER cone. The display normalization ([`Steadiness::from_cone_mult`]) maps the
/// widening multiplier to its narrowing complement `1 - cone_mult`: a `cone_mult` at the
/// no-narrowing baseline `1.0` (or worse, `> 1.0`) reads as an EMPTY bar, and a `cone_mult`
/// approaching `0.0` (a maximally steady, vanishingly-narrow cone) reads as a FULL bar. The
/// mapping is the complement of the widening multiplier — a single principled relation, NOT
/// a scattered magic literal — and the [`FillFraction`] clamp guards the `0..=1` range so a
/// degenerate `cone_mult` can never produce an out-of-range fill. Private inner + derived
/// [`Deref`].
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub(in crate::states::running::game::battlescape::status_panel) struct Steadiness(f32);

impl Steadiness {
    /// Derive the steadiness readout from the §1a [`ConeMult`] (the cone-widening
    /// multiplier).
    ///
    /// A steadier shot narrows the cone, so the steadiness is the narrowing complement of
    /// the widening multiplier: `1 - cone_mult`. A `cone_mult` at or above the baseline
    /// `1.0` (no narrowing) clamps to `0.0` (empty bar); a `cone_mult` toward `0.0`
    /// (maximally narrow) approaches `1.0` (full bar). The complement is clamped into
    /// `0.0..=1.0` so an out-of-range curve output can never produce a negative or
    /// over-full readout.
    #[must_use]
    pub(in crate::states::running::game::battlescape::status_panel) fn from_cone_mult(
        cone_mult: ConeMult,
    ) -> Self {
        Self((1.0 - *cone_mult).clamp(0.0, 1.0))
    }

    /// The steadiness as the named widget [`FillFraction`] the
    /// [`ProgressBar`](gdtf_ui::spawn_progress_bar) fill takes — the only boundary where the
    /// steadiness becomes a widget fill, and the [`FillFraction`] clamp re-guards the range.
    #[must_use]
    pub(in crate::states::running::game::battlescape::status_panel) const fn fill_fraction(
        self,
    ) -> FillFraction {
        FillFraction::new(self.0)
    }
}
