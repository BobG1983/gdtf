//! The sim **turn-cycle engine** (GTW-309) — the [`ActiveFaction`] resource, the pure
//! turn-start TU-regen helper, and the [`dispatch_end_turn`] system that drains the
//! [`EndTurnRequested`](crate::acts::EndTurnRequested) signal and cycles the turn.
//!
//! Two teams (gang `0` and gang `1`) alternate turns. On an
//! [`EndTurnRequested`](crate::acts::EndTurnRequested), [`dispatch_end_turn`] hands the
//! turn to the other team, regenerates that team's TU at its turn-start
//! ([`regen_team_tu`] — resetting [`Tu`](crate::ganger::Tu) to
//! [`TuMax`](crate::ganger::TuMax) for that team only, via the landed
//! [`reset_tu`](crate::tu::reset_tu) verb), and STOPS — exactly one advance per request
//! (GTW-70 removed the enemy auto-pass). When the player ends its turn, control genuinely
//! passes to the enemy and stays there: the GTW-70 enemy-AI brain
//! ([`enemy_ai_turn`](crate::ai::enemy_ai_turn)) drives the enemy turn and emits its OWN
//! [`EndTurnRequested`](crate::acts::EndTurnRequested) to hand control back to the player.
//! The turn-start budget refill is design canon (resolution.md §"What's pure math vs sim";
//! stats.md "A TU pool per turn").
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
