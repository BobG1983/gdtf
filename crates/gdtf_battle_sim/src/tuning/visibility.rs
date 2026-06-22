//! The GTW-338 squad fog-of-war visibility tunables — the sight radius and the
//! EXPLORED-memory modulate factor (`docs/combat/visibility.md` §"Tunables").
//!
//! These are **sim-authored** balance leaves: the squad fog is computed model-side
//! (`gdtf_battle_sim` owns the three VISIBLE / EXPLORED / UNSEEN states), so the two
//! coefficients live here with the rest of the combat tuning. They are **consumed by
//! the presenter** fog writer (GTW-342) — the model never renders the dim itself.

use bevy::prelude::Deref;
use serde::Deserialize;

/// The **view range** — one ganger's sight radius in **Chebyshev cells** (the 2D
/// disc that bounds per-ganger FOV before the level-aware LOS probe owns the height
/// axis; `docs/combat/visibility.md` §"Per-ganger FOV"). A ganger F sees `(cell,
/// level)` iff `Chebyshev(F.cell, cell) <= view_range` AND `has_los(..)`.
///
/// The default is `14`: the shipped **60×60** city gets a real fog horizon (the disc
/// is smaller than the map, so distant cells start UNSEEN), while the small **12×12**
/// fixtures read fully lit around the squad (the range exceeds the whole map, so every
/// cell is in reach). A distinct domain concept ⇒ its own newtype (no-bare-types rule):
/// a Chebyshev cell radius, never a bare `u16`. Private inner + derived [`Deref`];
/// `#[serde(transparent)]` lets it parse a bare RON scalar (the tuning-leaf precedent).
/// **Tunable** balance data — tests assert only its relation / parse, never the
/// magnitude.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ViewRange(u16);

impl ViewRange {
    /// Build a view range from its Chebyshev cell radius (tunable balance data).
    ///
    /// The constructor for the newtype — keeps the inner `u16` private (house style)
    /// while letting tests and any programmatic tuning edit build a range without a
    /// bare `u16` escaping; shipped values come from the `.ron` via the derived
    /// [`Deserialize`].
    #[must_use]
    pub const fn new(cells: u16) -> Self {
        Self(cells)
    }
}

impl Default for ViewRange {
    fn default() -> Self {
        // 14 Chebyshev cells — the 60×60 city gets a real fog horizon; the 12×12
        // fixtures read fully lit (the range exceeds the map). Tunable balance data;
        // value-agnostic tests only, never a pinned magnitude.
        Self(14)
    }
}

/// The **explored dim** — historically the modulate factor applied to EXPLORED terrain
/// (the rendered RGB × this, alpha untouched). A dimensionless value in `0..=1`.
///
/// **DEPRECATED / UNUSED by the renderer as of GTW-348.** The user changed the EXPLORED
/// treatment from "dimmer, same colour" to "same brightness, GREYSCALE" — colour-loss is
/// the memory cue, not brightness-loss (`docs/combat/visibility.md` §"Tunables"). The
/// presenter fog writer (GTW-342) no longer reads this factor: EXPLORED terrain renders at
/// full luminance, desaturated to greyscale, via the presenter's `TerrainFogMaterial`
/// `saturation` knob (`0.0` on EXPLORED), with no brightness scaling.
///
/// It is RETAINED (not removed) because GTW-348 is presenter-only — pruning it from
/// [`CombatTuning`](crate::CombatTuning), the shipped `tuning.ron`, and the sim parse /
/// default tests would ripple into the model and outside that scope. It stays a valid,
/// parseable tuning leaf so the model/RON contract is unbroken; a future ticket may either
/// repurpose it (e.g. a greyscale-floor offset) or retire it.
///
/// A distinct domain concept ⇒ its own newtype: a dimensionless RGB modulate factor, never
/// a bare `f32`. Private inner + derived [`Deref`]; `#[serde(transparent)]`. **Tunable**
/// balance data — tests assert only its range / parse, never the magnitude.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ExploredDim(f32);

impl ExploredDim {
    /// Build an explored-dim modulate factor from its magnitude (a dimensionless RGB
    /// scale in `0..=1`; tunable balance data).
    ///
    /// The constructor for the newtype — keeps the inner `f32` private (house style)
    /// while letting tests and any programmatic tuning edit build a factor without a
    /// bare `f32` escaping; shipped values come from the `.ron` via the derived
    /// [`Deserialize`].
    #[must_use]
    pub const fn new(factor: f32) -> Self {
        Self(factor)
    }
}

impl Default for ExploredDim {
    fn default() -> Self {
        // 0.55 — EXPLORED terrain's RGB × this (alpha untouched): dark enough to read
        // "memory, not live sight", bright enough to navigate by. Tunable balance data;
        // value-agnostic tests only, never a pinned magnitude.
        Self(0.55)
    }
}
