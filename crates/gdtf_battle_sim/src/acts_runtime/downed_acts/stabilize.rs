//! The **stabilize** verb + its shared `can_stabilize` predicate — an 8-adjacent
//! ALIVE ally halts the bleed clock by setting the [`Stabilized`] flag.

use super::reach::{Actor, DownedTarget, is_8_adjacent};
use crate::{
    ganger::{LifeState, Stabilized},
    tuning::{CombatTuning, StabilizeTu},
};

/// Whether `actor` can **stabilize** `target` — the shared guard for the HUD
/// Stabilize button and [`stabilize_downed`] (`docs/combat/resolution.md` §9).
///
/// True iff **all** hold:
///
/// 1. the two are [`is_8_adjacent`] (same-level Moore-8 reach),
/// 2. the actor is [`LifeState::Alive`] (a downed / dead actor cannot act),
/// 3. the target is [`LifeState::Downed`] (you stabilize the downed, not the
///    standing or the dead),
/// 4. `actor.faction == target.faction` — an **ally** (the faction differentiator;
///    an enemy can never stabilize), and
/// 5. the target is **not already stabilized** (its [`Stabilized`] is absent or
///    `Some(false)`; re-dressing a halted clock is a no-op).
///
/// Pure read over component values — no mutation, no draw, no pixel.
#[must_use]
pub fn can_stabilize(actor: &Actor, target: &DownedTarget) -> bool {
    is_8_adjacent(actor.pos, target.pos)
        && actor.life == LifeState::Alive
        && target.life == LifeState::Downed
        && actor.faction == target.faction
        && !target.stabilized.is_some_and(|s| *s)
}

/// **Stabilize** a Downed ganger if the shared [`can_stabilize`] guard passes — the
/// E3.8 §9 verb (`docs/combat/resolution.md` §9; `docs/combat/wounds-and-roster.md`
/// §"Downed → death … state machine").
///
/// When [`can_stabilize`] holds for `actor` over the `target` reads, this **sets**
/// the target's [`Stabilized`] flag to `Stabilized::new(true)` (mutating the passed
/// `&mut Stabilized` — the E3.7 flag is reused, this slice only sets it), so
/// [`crate::bleed::tick_bleed`] skips the ganger from the next round on. The target
/// **remains [`LifeState::Downed`]** — this verb never touches its [`LifeState`].
/// On success it returns `Some(`[`StabilizeTu`]`)`, the flat TU cost READ from
/// `tuning` (so the leaf is genuinely consulted); the cost is **not** debited from
/// any [`crate::ganger::Tu`] pool — that economy is **E4**.
///
/// When the guard is false (not adjacent, the actor not Alive, the target not Downed,
/// a cross-faction enemy, or the target already stabilized) this is a **no-op**:
/// it mutates nothing and returns `None` (predicate ⇔ act — the same guard the HUD
/// button reads). Pure, render-free mutation of a component reference — no pixel.
pub fn stabilize_downed(
    actor: &Actor,
    target: &DownedTarget,
    target_stabilized: &mut Stabilized,
    tuning: &CombatTuning,
) -> Option<StabilizeTu> {
    if !can_stabilize(actor, target) {
        return None;
    }
    // Set the E3.7 flag (reused, only set here) — the bleed clock halts; the ganger
    // stays Downed (this verb never writes LifeState).
    *target_stabilized = Stabilized::new(true);
    // READ the flat cost to wire the leaf — debiting a Tu pool is E4, not here.
    Some(tuning.stabilize_tu)
}
