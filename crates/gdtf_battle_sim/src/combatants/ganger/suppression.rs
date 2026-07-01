//! The **suppression** state a ganger under enemy fire carries — the [`Suppressed`]
//! marker component and the [`SuppressorCell`] directional anchor it holds (GTW-526,
//! child of GTW-41; `docs/combat/combat.md` "Suppression … advanced combat effects").
//!
//! When an OPPOSING ganger fires near a unit, that unit is **suppressed**: it becomes
//! a worse reactor (it cannot reaction-fire while pinned) and auto-drops behind its
//! nearest cover. The suppression is a per-ganger component (a suppressed unit carries
//! [`Suppressed`]; an un-suppressed one has no such component), holding the
//! [`SuppressorCell`] the fire came FROM — the directional anchor the auto-stance
//! system reads to decide which cover the unit ducks behind. Symmetric: BOTH factions
//! can be suppressed (a player unit under enemy fire, an enemy unit under player fire).
//!
//! A suppressed unit stays suppressed through its opponent's whole turn and clears at
//! its OWN turn-start (the faction-scoped `reset_suppression` cadence) — so the pin
//! lasts exactly as long as the ganger has not yet had a chance to act it off.

use bevy::prelude::{Component, Deref};

use crate::metric::CellLevel;

/// The `(cell, level)` a suppressing shot was fired FROM — the directional anchor a
/// [`Suppressed`] unit ducks away from.
///
/// A no-bare-types newtype over the suppressor's `(cell, level)` (a domain value — the
/// origin of the incoming fire, not framework plumbing): a private inner + a derived
/// [`Deref`] to [`CellLevel`], the crate's newtype house style. It is the SHOOTER's
/// own [`Position`](crate::ganger::Position) (the origin of the fire), NOT the aim cell
/// — so `Direction::from_cells(unit, *suppressor)` points from the pinned unit toward
/// the threat, which is the direction its cover should face.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SuppressorCell(CellLevel);

impl SuppressorCell {
    /// Build a suppressor-cell anchor from the shooter's `(cell, level)` origin.
    ///
    /// The public constructor (house style) so the producer system can build a
    /// `SuppressorCell` from the shooter's [`Position`](crate::ganger::Position)
    /// without reaching the private field.
    #[must_use]
    pub const fn new(from: CellLevel) -> Self {
        Self(from)
    }
}

/// Marks a ganger as **suppressed** by enemy fire — a per-ganger state component
/// (GTW-526).
///
/// A ganger under fire from within [`SuppressionRadius`](crate::tuning::SuppressionRadius)
/// of an OPPOSING shot's target carries this component; an un-suppressed ganger has none.
/// It holds the [`SuppressorCell`] the fire came from (the directional anchor the
/// auto-stance drop reads). Its two live effects this slice:
///
/// 1. **reaction-fire lockout** — a suppressed reactor is skipped in the
///    [`reaction_trigger`](crate::reaction::reaction_trigger) eligibility gate BEFORE the
///    interrupt roll, so a pinned unit cannot interrupt AND consumes zero
///    [`ReactionRng`](crate::rng::ReactionRng) draws (determinism-critical — the RNG
///    stream is unperturbed by suppression); and
/// 2. **auto-stance drop** — on a FRESH application the unit auto-drops behind its
///    nearest cover (Low → Prone, Mid/High → Crouching), written directly (no TU).
///
/// Applied by the producer on an opposing [`FireRequested`](crate::acts::FireRequested)
/// (a fresh application also emits
/// [`SuppressionApplied`](crate::suppression::SuppressionApplied); a re-application is an
/// idempotent refresh of the [`SuppressorCell`], no second signal, no stacking) and
/// cleared at the suppressed unit's OWN turn-start (`reset_suppression`, faction-scoped).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Suppressed {
    /// The `(cell, level)` the suppressing fire was fired FROM — the shooter's origin,
    /// the directional anchor the auto-stance drop ducks the unit away from.
    pub from: SuppressorCell,
}

impl Suppressed {
    /// Build a suppressed marker anchored to the suppressor's `(cell, level)` origin.
    ///
    /// The public constructor (house style) so the producer can build a `Suppressed`
    /// from the shooter's [`Position`](crate::ganger::Position) without reaching the
    /// private [`SuppressorCell`] inner.
    #[must_use]
    pub const fn new(from: SuppressorCell) -> Self {
        Self { from }
    }
}
