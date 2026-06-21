//! The sim **turn-cycle engine** (GTW-309) — the [`ActiveFaction`] resource, the pure
//! turn-start TU-regen helper, and the [`dispatch_end_turn`] system that drains the
//! [`EndTurnRequested`](crate::acts::EndTurnRequested) signal and cycles the turn.
//!
//! Two teams (gang `0` and gang `1`) alternate turns. On an
//! [`EndTurnRequested`](crate::acts::EndTurnRequested), [`dispatch_end_turn`] hands the
//! turn to the other team, regenerates that team's TU at its turn-start
//! ([`regen_team_tu`] — resetting [`Tu`](crate::ganger::Tu) to
//! [`TuMax`](crate::ganger::TuMax) for that team only, via the landed
//! [`reset_tu`](crate::tu::reset_tu) verb), and — while the enemy has no AI — auto-passes
//! the enemy turn straight back to the player, so [`ActiveFaction`] ends each cycle on the
//! [`PlayerFaction`](crate::battle::PlayerFaction). The turn-start budget refill is design
//! canon (resolution.md §"What's pure math vs sim"; stats.md "A TU pool per turn"); the
//! `TODO(AI)` seam in [`dispatch_end_turn`] marks where the enemy AI turn replaces the
//! auto-pass.
//!
//! ## Module map
//!
//! - [`active_faction`] — the [`ActiveFaction`] battle-lifetime resource (whose turn it
//!   is) + its [`ActiveFaction::advance`] two-team cycle.
//! - [`regen`] — the pure [`regen_team_tu`] turn-start TU-regen helper (a plain
//!   iterator-based fn, directly unit-testable without an `App`).
//! - [`dispatch`] — the [`dispatch_end_turn`] system: drains the
//!   [`EndTurnRequested`](crate::acts::EndTurnRequested) buffer and runs the turn cycle.
//! - [`test`] — the headless turn-cycle tests (the contract's three cases).

mod active_faction;
mod dispatch;
mod regen;

#[cfg(test)]
mod test;

pub use active_faction::ActiveFaction;
pub use dispatch::{TurnStarted, dispatch_end_turn};
pub use regen::regen_team_tu;
