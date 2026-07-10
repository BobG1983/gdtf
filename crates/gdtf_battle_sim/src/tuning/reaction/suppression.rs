//! The GTW-526 suppression tuning leaves — the two balance leaves that govern
//! how a fired shot pins opposing gangers (child of GTW-41).
//!
//! Split out of [`super::leaves`] (the GTW-466 reaction-fire substrate) because
//! suppression is a distinct feature/change-reason: the [`SuppressionRadius`]
//! (how wide the pin spreads from the shot's target) and the
//! [`SuppressionStabilityPenalty`] (how much steadiness a pinned shooter loses)
//! are authored under `reaction:` in `assets/core_tuning/combat.tuning.ron` and
//! carried as fields of [`ReactionTuning`](super::ReactionTuning), but they are
//! consumed by the suppression pipeline (`crate::suppression`) and the aim
//! composer (`crate::aim::stability_for`), not the reaction-cap functions.

use bevy::prelude::Deref;
use serde::Deserialize;

/// The **suppression radius** — how many cells out from a shot's target an opposing
/// ganger is suppressed by the fire (GTW-526, child of GTW-41).
///
/// A shot's suppression covers every OPPOSING-faction ganger within this Chebyshev
/// radius of the shot's target cell/level: radius `0` suppresses ONLY the directly
/// targeted occupant cell; a larger radius pins a wider bystander area around the
/// impact. Default `1` — a defensible-but-arbitrary starting point (a fired shot pins
/// the target cell plus its immediate Moore-8 neighbours), mirroring the
/// [`ReactionCapBase`](super::ReactionCapBase) precedent; tests assert only that the
/// leaf parses, never this magnitude. `#[serde(transparent)]` lets it parse a bare RON
/// scalar; private inner + derived [`Deref`] over the `u8` cell count.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct SuppressionRadius(u8);

impl SuppressionRadius {
    /// Build a suppression radius from its cell count (a starting point, TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `u8` private (house style)
    /// while letting tests and any programmatic tuning edit build a value without a
    /// bare `u8` escaping; shipped values come from the `.ron` via the derived
    /// [`Deserialize`].
    #[must_use]
    pub const fn new(radius: u8) -> Self {
        Self(radius)
    }
}

impl Default for SuppressionRadius {
    fn default() -> Self {
        // 1 — a STARTING POINT (tunable balance data): a fired shot pins the target
        // cell plus its immediate Moore-8 neighbours. Value-agnostic tests only, never
        // a pinned magnitude.
        Self(1)
    }
}

/// The **suppression stability penalty** — the number of stability-score points a
/// [`Suppressed`](crate::ganger::Suppressed) shooter LOSES, widening its dispersion cone
/// (GTW-526, child of GTW-41; `docs/combat/combat.md` "Suppression … makes the target …
/// a worse shot").
///
/// A pinned shooter shoots worse: this **positive** magnitude is negated by the composer
/// ([`stability_for`](crate::aim::stability_for)) into a negative
/// [`SuppressionStability`](crate::stability::SuppressionStability) contribution, lowering
/// the 0–100 stability score so the cone-mult curve reads a *higher* multiplier (a wider
/// cone). Default `40.0` — a defensible-but-arbitrary starting point (roughly the prone
/// stance's steadiness, so heavy suppression can undo a careful posture), mirroring the
/// [`SuppressionRadius`] / [`ReactionCapBase`](super::ReactionCapBase) precedent; tests
/// assert only the LOWER-stability / WIDER-cone invariant, never this magnitude.
/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner + derived
/// [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct SuppressionStabilityPenalty(f32);

impl SuppressionStabilityPenalty {
    /// Build a suppression stability penalty from its point magnitude (a starting point,
    /// TBD tuning).
    ///
    /// The constructor for the newtype — keeps the inner `f32` private (house style)
    /// while letting tests and any programmatic tuning edit build a value without a bare
    /// `f32` escaping; shipped values come from the `.ron` via the derived
    /// [`Deserialize`]. The magnitude is stored **positive** (the size of the penalty);
    /// the composer negates it into a subtractive stability contribution.
    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }
}

impl Default for SuppressionStabilityPenalty {
    fn default() -> Self {
        // 40.0 — a STARTING POINT (tunable balance data): a suppressed shooter loses ~40
        // stability points (about the prone-stance contribution), so heavy suppression
        // undoes a careful posture. Value-agnostic tests only, never a pinned magnitude.
        Self(40.0)
    }
}
