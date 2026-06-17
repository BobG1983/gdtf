//! The battle-local ammo STATE: the [`Magazine`] current-rounds component, its
//! saturating per-round decrement, and the [`clamp_burst`] burst-clamp primitive.

use bevy::prelude::{Component, Deref};

use crate::weapon::{MagazineSize, ModeShots};

/// A ganger's **magazine** — the rounds currently loaded in their weapon.
///
/// The battle-local AMMO state (resolution.md §"What's pure math vs sim": the
/// "ammo clamp" `fire()` honors): the magazine's *capacity* is a weapon NUMBER
/// ([`MagazineSize`](crate::weapon::MagazineSize)), while the rounds *currently*
/// loaded are this per-ganger state, clamped at construction by that capacity
/// (never exceeds it). [`spend_round`](Magazine::spend_round) drains it one round
/// at a time (saturating); [`clamp_burst`] reads it to bound a burst's shot count.
///
/// A `u16` count (matching [`MagazineSize`](crate::weapon::MagazineSize)). Private
/// inner + derived [`Deref`]; a distinct Component so a shot / HUD system can query
/// `&Magazine` alone. Build it with [`Magazine::loaded`] (full) or
/// [`Magazine::new`] (a partial load, clamped). Defaults to `0` (empty — a fresh
/// ganger carries no ammo until the situation setup loads a magazine; a structural
/// spawn default, not a balance value).
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Magazine(u16);

impl Magazine {
    /// Build a magazine loaded with `rounds`, **clamped** to the weapon's
    /// [`MagazineSize`](crate::weapon::MagazineSize) capacity.
    ///
    /// The general constructor: a request above capacity is clamped down to
    /// capacity (`min(rounds, *size)`) — the magazine can never hold more than its
    /// weapon's size — and a request within capacity is preserved exactly
    /// (AC1). Use [`loaded`](Magazine::loaded) for a full magazine.
    #[must_use]
    pub fn new(rounds: u16, size: MagazineSize) -> Self {
        Self(rounds.min(*size))
    }

    /// Build a **full** magazine — loaded to the weapon's
    /// [`MagazineSize`](crate::weapon::MagazineSize) capacity.
    ///
    /// The convenience constructor for a freshly-reloaded weapon at full capacity
    /// ([`new`](Magazine::new) with `rounds == *size`).
    #[must_use]
    pub fn loaded(size: MagazineSize) -> Self {
        Self(*size)
    }

    /// Spend **one** round — a **saturating** decrement that floors at `0`.
    ///
    /// The per-round primitive the E4.5 `fire()` burst loop calls once per fired
    /// round (resolution.md §"What's pure math vs sim": `fire()` "per-round spend").
    /// Uses [`u16::saturating_sub`]: spending a round from an already-empty
    /// magazine leaves it at `0` — **never** underflows / wraps to `~65535` (AC2).
    pub const fn spend_round(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }
}

/// Clamp a fire mode's shot count to the rounds actually in the magazine — the
/// burst-clamp primitive the E4.5 `fire()` burst loop runs.
///
/// `fire()` can never loop more rounds than are loaded (resolution.md §"What's pure
/// math vs sim": the "ammo clamp"): the looped count is `min(mode shots, rounds
/// left)`. Returns a [`ModeShots`](crate::weapon::ModeShots) so the bounded count
/// keeps its per-mode meaning. With an empty magazine the result is `0` (no round
/// fires); within ammo the mode's full shot count passes through unchanged.
#[must_use]
pub fn clamp_burst(shots: ModeShots, mag: &Magazine) -> ModeShots {
    ModeShots::new((*shots).min(**mag))
}
