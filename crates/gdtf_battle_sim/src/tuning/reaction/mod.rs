//! The §8 reaction-fire layer — tuning leaves (GTW-466) + the deterministic
//! opposed-check core (GTW-467).
//!
//! `docs/combat/resolution.md` §8 designs reaction fire as "not yet built" but
//! fully specifies its contract:
//!
//! ```text
//! score        = Reactions × (TU_left / TU_max)
//! P(interrupt) = score_watcher / (score_watcher + score_mover)
//! max interrupts this enemy turn = cap(Reactions)   (tunable)
//! ```
//!
//! plus a probability clamp (`p_min`/`p_max`) so the extremes are never an
//! absolute 0%/100%, and the rule that **no ganger is ever hard-locked out**.
//!
//! This dir-module splits along its two landed slices (each within the
//! warn-300/block-400 size cap), wiring them here:
//!
//! - [`leaves`] — the GTW-466 **data substrate**: the four tuning leaves
//!   ([`ReactionCapBase`] / [`ReactionCapPerReactions`] / [`ReactionPMin`] /
//!   [`ReactionPMax`]), the [`ReactionTuning`] group, and the two pure tuning
//!   functions [`reaction_cap`] (the per-ganger interrupt cap) and
//!   [`clamp_probability`] (the `[p_min, p_max]` clamp).
//! - [`core`] — the GTW-467 **deterministic opposed-check core**: the output
//!   newtypes ([`ReactionScore`] / [`ReactionProbability`] / [`ReactionsUsed`])
//!   and the four pure functions ([`reaction_score`] / [`interrupt_probability`]
//!   / [`rolls_interrupt`] / [`may_interrupt`]).
//!
//! Everything here is **pure functions over plain data + injected seeded RNG** —
//! no systems, no `&mut World`, no ECS trigger. The live trigger (interrupt an
//! enemy acting in LOS, advance the per-turn counter at the turn boundary) is
//! GTW-468.

mod core;
mod leaves;

#[cfg(test)]
mod tests;

pub use core::{
    Interrupts, MayInterrupt, ReactionProbability, ReactionScore, ReactionsUsed,
    interrupt_probability, may_interrupt, reaction_score, rolls_interrupt,
};

pub use leaves::{
    ReactionCap, ReactionCapBase, ReactionCapPerReactions, ReactionPMax, ReactionPMin,
    ReactionTuning, SuppressionRadius, SuppressionStabilityPenalty, clamp_probability,
    reaction_cap,
};
