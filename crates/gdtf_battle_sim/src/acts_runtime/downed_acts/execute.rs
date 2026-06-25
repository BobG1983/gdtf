//! The **execute** verb + its shared `can_execute` predicate — an 8-adjacent ALIVE
//! enemy finishes a Downed ganger outright.

use super::reach::{Actor, DownedTarget, is_8_adjacent};
use crate::{
    ganger::LifeState,
    tuning::{CombatTuning, ExecuteTu},
};

/// Whether `actor` can **execute** `target` — the shared guard for the HUD Execute
/// button and [`execute_downed`] (`docs/combat/resolution.md` §9).
///
/// True iff **all** hold:
///
/// 1. the two are [`is_8_adjacent`] (same-level Moore-8 reach),
/// 2. the actor is [`LifeState::Alive`],
/// 3. the target is [`LifeState::Downed`], and
/// 4. `actor.faction != target.faction` — an **enemy** (the faction differentiator;
///    an ally can never execute).
///
/// The target's [`Stabilized`](crate::ganger::Stabilized) flag does **not** gate
/// execute — a stabilized Downed ganger can still be finished off by an enemy. Pure
/// read over component values — no mutation, no draw, no pixel.
#[must_use]
pub fn can_execute(actor: &Actor, target: &DownedTarget) -> bool {
    is_8_adjacent(actor.pos, target.pos)
        && actor.life == LifeState::Alive
        && target.life == LifeState::Downed
        && actor.faction != target.faction
}

/// **Execute** a Downed ganger if the shared [`can_execute`] guard passes — the
/// E3.8 §9 verb (`docs/combat/resolution.md` §9; `docs/combat/wounds-and-roster.md`
/// §"Downed → death … state machine").
///
/// When [`can_execute`] holds for `actor` over the `target` reads, this transitions
/// the target outright to [`LifeState::Dead`] (mutating the passed `&mut LifeState`)
/// — the enemy finisher. On success it returns `Some(`[`ExecuteTu`]`)`, the flat TU
/// cost READ from `tuning` (so the leaf is genuinely consulted); the cost is **not**
/// debited from any [`crate::ganger::Tu`] pool — that economy is **E4**.
///
/// When the guard is false (not adjacent, the actor not Alive, the target not Downed,
/// or a same-faction ally) this is a **no-op**: it mutates nothing and returns `None`
/// (predicate ⇔ act — the same guard the HUD button reads). Pure, render-free
/// mutation of a component reference — no pixel.
pub fn execute_downed(
    actor: &Actor,
    target: &DownedTarget,
    target_life: &mut LifeState,
    tuning: &CombatTuning,
) -> Option<ExecuteTu> {
    if !can_execute(actor, target) {
        return None;
    }
    // Finish the Downed ganger outright — the enemy executor.
    *target_life = LifeState::Dead;
    // READ the flat cost to wire the leaf — debiting a Tu pool is E4, not here.
    Some(tuning.execute_tu)
}
