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
//! - the GTW-355 [`ReactionShotFired`](crate::acts::movement::ReactionShotFired) message — this
//!   module is its long-promised PRODUCER (closing the GTW-355 orphan): it halts a walking
//!   actor at its current cell on a successful interrupt.
//!
//! ## Module map
//!
//! - `snapshot` — the reaction-relevant ganger snapshot shape (the Copy `ReactionRow`,
//!   its deterministic ordering keys, and the read-only snapshot query alias);
//! - `trigger` — the [`reaction_trigger`] system (observe the act-in-LOS surface → gate
//!   eligible reactors in deterministic order → delegate each pair);
//! - `interrupt` — the per-(actor, reactor) evaluation (the C2 eligibility gates → the
//!   C3 opposed check → the C4 fire + halt + count emission);
//! - `ledger` — the per-pass pending-spend ledger (GTW-646): the working TU / facing /
//!   magazine view that advances as interrupts are emitted, so a later same-pass
//!   interrupt is gated on the state the dispatcher will actually see (the cap spend
//!   stays 1:1 with dispatched shots);
//! - `reset` — the turn-boundary [`reset_reactions_used`] system (zero every watcher's
//!   per-turn counter on a [`TurnStarted`](crate::turn::TurnStarted)).
//!
//! All param-only — `Query` / `Res` / `ResMut` / `MessageReader` / `MessageWriter` — NO
//! `&mut World` (`bevy-traps.md` #7).
//!
//! The schedule WIRING lives in [`SimActsPlugin`](crate::acts::SimActsPlugin); see
//! [`reaction_trigger`]'s docs for the C5 ordering (and the cycle constraint it resolves).

mod interrupt;
mod ledger;
mod reset;
mod snapshot;
mod trigger;

#[cfg(test)]
mod test;

pub use reset::reset_reactions_used;
pub use trigger::reaction_trigger;
