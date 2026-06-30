//! The §8 reaction-fire tuning leaves — cap inputs and probability clamp
//! (GTW-466 data substrate).
//!
//! Resolution.md §8 designs the reaction-fire system as "not yet built" but
//! fully specifies its tuning contract:
//!
//! ```text
//! max interrupts this enemy turn = cap(Reactions)   (tunable)
//! ```
//!
//! and an optional **probability clamp** (`p_min` / `p_max`) so the opposed-check
//! probability never collapses to a hard 0% or 100%. This module is the **data
//! substrate** (GTW-466 child A): the four tuning leaves + the per-ganger cap and
//! clamp pure functions. The deterministic opposed-check core that consumes them
//! is GTW-467 ([`super::core`]); the live trigger is GTW-468.
//!
//! ## DESIGN FORK — cap formula (resolution.md §8, line 192)
//!
//! Resolution.md §8 leaves the exact `cap()` formula TBD. For this substrate the
//! formula is:
//!
//! ```text
//! reaction_cap = floor(cap_base + cap_per_reactions × Reactions)
//! ```
//!
//! The coefficients are the [`ReactionCapBase`] / [`ReactionCapPerReactions`]
//! tunable leaves. Defaults are **defensible-but-arbitrary** starting points,
//! following the `ActCadence::DEFAULT` precedent — balance data, never pinned by
//! a magnitude test. Tests assert only **invariants** (monotone, clamp-edge
//! boundary), never magnitudes.

use bevy::prelude::Deref;
use serde::Deserialize;

use crate::ganger::Reactions;

// ── Tuning leaves ────────────────────────────────────────────────────────────

/// The **reaction-cap base constant** — the additive floor of the per-ganger
/// interrupt cap (`docs/combat/resolution.md` §8:
/// `max interrupts this enemy turn = cap(Reactions)`; the exact formula is
/// `floor(cap_base + cap_per_reactions × Reactions)`).
///
/// Even a ganger with zero [`Reactions`] may still gain one interrupt if
/// `cap_base ≥ 1.0` (a guaranteed minimum cap that keeps reaction fire alive
/// for low-Reactions gangers). Default `1.0` — a defensible-but-arbitrary
/// starting point mirroring the `ActCadence::DEFAULT` precedent; tests assert
/// only the monotone invariant, never this magnitude.
/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner +
/// derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReactionCapBase(f32);

impl ReactionCapBase {
    /// Build a reaction-cap base constant from its magnitude (a starting point,
    /// TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `f32` private (house
    /// style) while letting tests and any programmatic tuning edit build a value
    /// without a bare `f32` escaping; shipped values come from the `.ron` via
    /// the derived [`Deserialize`].
    #[must_use]
    pub const fn new(base: f32) -> Self {
        Self(base)
    }
}

impl Default for ReactionCapBase {
    fn default() -> Self {
        // A base of 1.0 — a STARTING POINT (tunable balance data), so even a
        // zero-Reactions ganger can still react at least once. Value-agnostic
        // tests only, never a pinned magnitude.
        Self(1.0)
    }
}

/// The **reaction-cap per-Reactions coefficient** — how much each unit of
/// [`Reactions`] adds to the per-ganger interrupt cap
/// (`docs/combat/resolution.md` §8: `cap(Reactions)`, tunable).
///
/// In the formula `floor(cap_base + cap_per_reactions × Reactions)`, this is
/// the slope: a larger coefficient means a high-Reactions ganger can interrupt
/// many more times per enemy turn. Default `0.5` — a defensible-but-arbitrary
/// starting point; tests assert only the monotone invariant, never this
/// magnitude. `#[serde(transparent)]` lets it parse a bare RON scalar; private
/// inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReactionCapPerReactions(f32);

impl ReactionCapPerReactions {
    /// Build a per-Reactions cap coefficient from its magnitude (a starting
    /// point, TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `f32` private (house
    /// style) while letting tests and any programmatic tuning edit build a value
    /// without a bare `f32` escaping; shipped values come from the `.ron` via
    /// the derived [`Deserialize`].
    #[must_use]
    pub const fn new(per_reactions: f32) -> Self {
        Self(per_reactions)
    }
}

impl Default for ReactionCapPerReactions {
    fn default() -> Self {
        // 0.5 per unit of Reactions — a STARTING POINT (tunable balance data;
        // every whole unit of Reactions adds 0.5 to the cap before floor).
        // Value-agnostic tests only, never a pinned magnitude.
        Self(0.5)
    }
}

/// The **reaction probability minimum clamp** — the floor applied to the
/// opposed-check `P(interrupt)` so it never collapses to an absolute 0%
/// (`docs/combat/resolution.md` §8: "optional probability clamp `p_min`/`p_max`,
/// e.g. 0.05 / 0.95 so the extremes are never an absolute 0% or 100%").
///
/// Any computed probability BELOW this value is clamped UP to `p_min`  — no
/// ganger is ever completely locked out of reacting. Must satisfy
/// `0.0 ≤ p_min < p_max ≤ 1.0`. Default `0.05` (the resolution.md §8 example
/// value). `#[serde(transparent)]` lets it parse a bare RON scalar; private
/// inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReactionPMin(f32);

impl ReactionPMin {
    /// Build a reaction probability minimum clamp from its magnitude (the
    /// resolution.md §8 example value is 0.05; a starting point, TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `f32` private (house
    /// style); shipped values come from the `.ron` via the derived
    /// [`Deserialize`].
    #[must_use]
    pub const fn new(p_min: f32) -> Self {
        Self(p_min)
    }
}

impl Default for ReactionPMin {
    fn default() -> Self {
        // 0.05 — the resolution.md §8 example value (line 165: "e.g. 0.05 / 0.95").
        // A STARTING POINT (tunable balance data). Value-agnostic tests only,
        // never a pinned magnitude.
        Self(0.05)
    }
}

/// The **reaction probability maximum clamp** — the ceiling applied to the
/// opposed-check `P(interrupt)` so it never reaches an absolute 100%
/// (`docs/combat/resolution.md` §8: "optional probability clamp `p_min`/`p_max`,
/// e.g. 0.05 / 0.95 so the extremes are never an absolute 0% or 100%").
///
/// Any computed probability ABOVE this value is clamped DOWN to `p_max` — a
/// very-high-Reactions ganger still gives the mover a fighting chance. Must
/// satisfy `0.0 ≤ p_min < p_max ≤ 1.0`. Default `0.95` (the resolution.md §8
/// example value). `#[serde(transparent)]` lets it parse a bare RON scalar;
/// private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ReactionPMax(f32);

impl ReactionPMax {
    /// Build a reaction probability maximum clamp from its magnitude (the
    /// resolution.md §8 example value is 0.95; a starting point, TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `f32` private (house
    /// style); shipped values come from the `.ron` via the derived
    /// [`Deserialize`].
    #[must_use]
    pub const fn new(p_max: f32) -> Self {
        Self(p_max)
    }
}

impl Default for ReactionPMax {
    fn default() -> Self {
        // 0.95 — the resolution.md §8 example value (line 165: "e.g. 0.05 / 0.95").
        // A STARTING POINT (tunable balance data). Value-agnostic tests only,
        // never a pinned magnitude.
        Self(0.95)
    }
}

// ── Group struct ─────────────────────────────────────────────────────────────

/// The §8 reaction-fire tuning group — the cap formula inputs and probability
/// clamp (`docs/combat/resolution.md` §8).
///
/// Bundles the four leaves that govern how many times a ganger may react on an
/// enemy turn ([`ReactionCapBase`] / [`ReactionCapPerReactions`]) and what
/// probability bounds keep the opposed check away from 0%/100%
/// ([`ReactionPMin`] / [`ReactionPMax`]). Used as a field of
/// [`crate::tuning::CombatTuning`]; authored in
/// `assets/core_tuning/combat.tuning.ron` under `reaction:`.
///
/// The GTW-467 opposed-check core reads these leaves through [`reaction_cap`],
/// [`clamp_probability`], and
/// [`interrupt_probability`](super::core::interrupt_probability); the live
/// trigger is GTW-468.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
pub struct ReactionTuning {
    /// The additive floor of the cap formula: `floor(cap_base + …)`.
    /// Higher values give even low-Reactions gangers more interrupts.
    pub cap_base:          ReactionCapBase,
    /// The per-[`Reactions`] slope of the cap formula: `… + cap_per_reactions × Reactions`.
    /// Higher values make high-Reactions gangers dramatically more dangerous.
    pub cap_per_reactions: ReactionCapPerReactions,
    /// The probability floor: no `P(interrupt)` may fall below this value.
    /// Resolution.md §8 example: `0.05`.
    pub p_min:             ReactionPMin,
    /// The probability ceiling: no `P(interrupt)` may exceed this value.
    /// Resolution.md §8 example: `0.95`.
    pub p_max:             ReactionPMax,
}

// ── Pure functions ────────────────────────────────────────────────────────────

/// Cast a non-negative floored `f32` to `u32`.
///
/// Caller contract: `val` is already the result of `f32::floor()` on a value
/// that was guarded `>= 0.0` before this call — so it is non-negative, has no
/// fractional part, and fits a `u32` for any sane tuning magnitude.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the caller guards val >= 0.0 and applies floor() before calling here, \
              so the cast is always lossless for any sane tuning magnitude"
)]
const fn floor_to_u32(val: f32) -> u32 {
    val as u32
}

/// The per-ganger **reaction interrupt cap** — the maximum number of times a
/// ganger may interrupt the enemy's turn with reaction fire.
///
/// Formula (DESIGN FORK — resolution.md §8 line 192 leaves this TBD): `floor(
/// cap_base + cap_per_reactions × Reactions)`, where the coefficients are the
/// [`ReactionTuning`] leaves. The result is a `u32` count (the floor of a
/// non-negative quantity — `Reactions` is non-negative by construction so the
/// result is always ≥ 0; clamped to zero if the float is negative to stay
/// safe against extreme tuning edits).
///
/// Consumed by [`may_interrupt`](super::core::may_interrupt) (GTW-467) as the
/// per-turn ceiling; the live trigger is GTW-468.
///
/// # Arguments
///
/// - `reactions` — the ganger's [`Reactions`] computed stat (derived at setup
///   via [`crate::ganger::derive_stats`]).
/// - `tuning` — the [`ReactionTuning`] group from [`crate::tuning::CombatTuning`].
#[must_use]
pub fn reaction_cap(reactions: Reactions, tuning: &ReactionTuning) -> u32 {
    // FMA form: cap_per_reactions × reactions + cap_base (suboptimal_flops lint).
    let raw = (*tuning.cap_per_reactions).mul_add(*reactions, *tuning.cap_base);
    // Clamp to 0 before the floor so a badly-tuned negative never wraps.
    if raw < 0.0 {
        0
    } else {
        floor_to_u32(raw.floor())
    }
}

/// The **probability clamp** — pins the opposed-check `P(interrupt)` to the
/// `[p_min, p_max]` interval from [`ReactionTuning`].
///
/// Resolution.md §8 (line 165): "an optional probability clamp (`p_min`/`p_max`,
/// e.g. 0.05 / 0.95 so the extremes are never an absolute 0% or 100%)".
///
/// - A value below `p_min` is clamped UP to `p_min`.
/// - A value above `p_max` is clamped DOWN to `p_max`.
/// - A value already in `[p_min, p_max]` is returned unchanged.
///
/// The GTW-467 [`interrupt_probability`](super::core::interrupt_probability)
/// routes its raw ratio through this clamp; the live trigger is GTW-468.
///
/// # Arguments
///
/// - `p` — the raw computed probability (any `f32`; values outside `[0, 1]` are
///   valid inputs — the clamp enforces the bounds).
/// - `tuning` — the [`ReactionTuning`] group from [`crate::tuning::CombatTuning`].
#[must_use]
pub fn clamp_probability(p: f32, tuning: &ReactionTuning) -> f32 {
    p.clamp(*tuning.p_min, *tuning.p_max)
}
