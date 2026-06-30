//! The **live reaction-fire trigger** (GTW-468, the final child of GTW-38) — the ECS
//! wiring that makes `docs/combat/resolution.md` §8 reaction fire real in play.
//!
//! When a ganger ACTS in an opposing reactor's line of sight — it completes a movement
//! STEP or a FIRE act — every eligible, sighted reactor runs the GTW-467 opposed check
//! ([`reaction_score`](crate::tuning::reaction_score) →
//! [`interrupt_probability`](crate::tuning::interrupt_probability) →
//! [`rolls_interrupt`](crate::tuning::rolls_interrupt)); on success the reactor fires a
//! NORMAL interrupt shot from its unspent TU, consuming its per-turn cap
//! ([`may_interrupt`](crate::tuning::may_interrupt)). There is **no overwatch toggle** —
//! reaction fire is automatic, and the §8 probability clamp guarantees no ganger is ever
//! hard-locked out.
//!
//! ## What this module REUSES (it reimplements nothing)
//!
//! - the GTW-467 opposed-check core ([`reaction_score`](crate::tuning::reaction_score) /
//!   [`interrupt_probability`](crate::tuning::interrupt_probability) /
//!   [`rolls_interrupt`](crate::tuning::rolls_interrupt) /
//!   [`may_interrupt`](crate::tuning::may_interrupt)) — called verbatim;
//! - the faction-agnostic engagement gate [`can_see`](crate::los::can_see) + the shared
//!   arc verdict [`can_engage`](crate::acts::can_engage) — gate each `(reactor, actor)`
//!   pair so a fired interrupt is dispatcher-accepted;
//! - the existing fire act: it emits a [`FireRequested`](crate::acts::FireRequested) the
//!   landed [`dispatch_fire`](crate::acts::dispatch_fire) → `fire()` resolves as a normal
//!   shot (full dispersion + damage, **no reaction damage modifier**) AND spends the
//!   reactor's TU — so this module NEVER charges TU or rolls a shot itself;
//! - the GTW-355 [`ReactionShotFired`](crate::move_acts::ReactionShotFired) message — this
//!   module is its long-promised PRODUCER (closing the GTW-355 orphan): it halts a walking
//!   actor at its current cell on a successful interrupt.
//!
//! ## Module map
//!
//! - `trigger` — the [`reaction_trigger`] system (observe the act-in-LOS surface → gate
//!   eligible reactors → run the opposed check → fire + halt + count) and the turn-boundary
//!   [`reset_reactions_used`] system (zero every watcher's per-turn counter on a
//!   [`TurnStarted`](crate::turn::TurnStarted)). Param-only — `Query` / `Res` / `ResMut` /
//!   `MessageReader` / `MessageWriter` — NO `&mut World` (`bevy-traps.md` #7).
//!
//! The schedule WIRING lives in [`SimActsPlugin`](crate::acts::SimActsPlugin); see
//! [`reaction_trigger`]'s docs for the C5 ordering (and the cycle constraint it resolves).

mod trigger;

#[cfg(test)]
mod test;

pub use trigger::{reaction_trigger, reset_reactions_used};
