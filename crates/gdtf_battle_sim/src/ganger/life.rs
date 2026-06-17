//! The terminal life-state machine — the [`LifeState`] two-pool outcome and the
//! [`Stabilized`] bleed-out flag.

use bevy::prelude::{Component, Deref};
use serde::Deserialize;

/// A ganger's terminal life state — the two-pool outcome machine.
///
/// The combat/death system owns this (it is a state component, not a stat):
/// `HP ≤ 0` → [`Downed`](LifeState::Downed) (alive, incapacitated, dying),
/// `Wounds ≤ 0` → [`Dead`](LifeState::Dead) (Dead trumps Downed). It drives
/// occupancy updates in E1.7 (a corpse / downed body frees or holds its cell
/// differently). A standalone named enum component (wounds-and-roster.md state
/// machine). Defaults to [`Alive`](LifeState::Alive).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum LifeState {
    /// Up and fighting — full agency.
    #[default]
    Alive,
    /// HP gone, Wounds remaining — incapacitated but alive (bleeding out unless
    /// stabilized; can be executed or recovered).
    Downed,
    /// Wounds gone — dead in battle (from stacked injuries or one Fatal hit).
    Dead,
}

/// Whether a [`LifeState::Downed`] ganger has been **stabilized** — its bleed-out
/// clock halted.
///
/// The downed-ganger "stabilized, skip the bleed clock" flag
/// (`docs/combat/resolution.md` §9; `docs/combat/wounds-and-roster.md`
/// §"Downed → death … state machine"): an 8-adjacent ally's stabilize action
/// **sets** this `true`, after which the per-round bleed-out drain
/// ([`crate::bleed::tick_bleed`]) skips the ganger — no new *Bleeding Out* stacks,
/// the Wounds already drained stay drained, and the ganger **remains Downed**
/// (alive, out for the rest of the mission).
///
/// This slice (E3.7) is the flag's single **home**: [`tick_bleed`](crate::bleed::tick_bleed)
/// must **read** it to skip stabilized gangers, so it is defined here, not deferred
/// to the E3.8 stabilize action (which only **sets** this already-defined flag). A
/// distinct component so the bleed-out path can query `Option<&Stabilized>` alone.
/// Defaults to `false` (a freshly-downed ganger is **not** stabilized — the clock
/// runs until an ally dresses the wound; a structural spawn default, not a balance
/// value). Private inner + derived [`Deref`], house style.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Stabilized(pub(super) bool);

impl Stabilized {
    /// Build a stabilized flag — `true` once an ally has dressed the downed
    /// ganger's wound (the clock halts), `false` while it still bleeds.
    ///
    /// The public constructor (private inner + constructor, the crate's newtype
    /// house style) so the E3.8 `stabilize_downed` action can set the flag, and the
    /// E3.7 bleed tests can spawn a stabilized ganger, without reaching the private
    /// field.
    #[must_use]
    pub const fn new(stabilized: bool) -> Self {
        Self(stabilized)
    }
}
