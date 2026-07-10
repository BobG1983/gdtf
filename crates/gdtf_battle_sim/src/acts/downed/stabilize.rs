//! The **stabilize** verb + its shared `can_stabilize` predicate — an 8-adjacent
//! ALIVE ally halts the bleed clock by removing the
//! [`BleedingOut`](crate::effects::bleed::BleedingOut) condition (GTW-695).

use bevy::prelude::{Commands, Deref, Entity};

use super::reach::{Actor, DownedTarget, is_8_adjacent};
use crate::{
    effects::bleed::BleedingOut,
    ganger::LifeState,
    tuning::{CombatTuning, StabilizeTu},
};

/// Whether an actor may **stabilize** a Downed target — the [`can_stabilize`] guard
/// verdict the HUD Stabilize button and the act share (`docs/combat/resolution.md` §9).
///
/// `true` means an ally dressing is legal against this target; `false` means it is not.
/// A distinct act-permission predicate, not a bare `bool`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanStabilize(bool);

impl CanStabilize {
    /// Build the stabilize-permission verdict from the computed guard.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

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
/// 5. the target is **currently [`BleedingOut`]** (its condition marker is present;
///    a stabilized target no longer carries it, so re-dressing a halted clock is a
///    guarded no-op).
///
/// Pure read over component values — no mutation, no draw, no pixel.
#[must_use]
pub fn can_stabilize(actor: &Actor, target: &DownedTarget) -> CanStabilize {
    CanStabilize::new(
        *is_8_adjacent(actor.pos, target.pos)
            && actor.life == LifeState::Alive
            && target.life == LifeState::Downed
            && actor.faction == target.faction
            && target.bleeding_out.is_some(),
    )
}

/// **Stabilize** a Downed ganger if the shared [`can_stabilize`] guard passes — the
/// E3.8 §9 verb (`docs/combat/resolution.md` §9; `docs/combat/wounds-and-roster.md`
/// §"Downed → death … state machine").
///
/// When [`can_stabilize`] holds for `actor` over the `target` reads, this **removes**
/// the target's [`BleedingOut`] condition (via `commands` on `target_entity`), so
/// [`crate::effects::bleed::tick_bleed`] skips the ganger from the next round on (GTW-695 — the
/// condition is reified, not a negation flag). The target **remains
/// [`LifeState::Downed`]** — this verb never touches its [`LifeState`]. On success it
/// returns `Some(`[`StabilizeTu`]`)`, the flat TU cost READ from `tuning` (so the leaf
/// is genuinely consulted); the cost is **not** debited from any [`crate::ganger::Tu`]
/// pool — that economy is **E4**.
///
/// When the guard is false (not adjacent, the actor not Alive, the target not Downed,
/// a cross-faction enemy, or the target not currently bleeding out — already stabilized)
/// this is a **no-op**: it removes nothing and returns `None` (predicate ⇔ act — the
/// same guard the HUD button reads). Removal is via `commands` (deferred); the ganger
/// stays Downed. Render-free model logic — no pixel.
pub fn stabilize_downed(
    actor: &Actor,
    target: &DownedTarget,
    target_entity: Entity,
    commands: &mut Commands,
    tuning: &CombatTuning,
) -> Option<StabilizeTu> {
    if !*can_stabilize(actor, target) {
        return None;
    }
    // Remove the §9 BleedingOut condition — the bleed clock halts; the ganger stays
    // Downed (this verb never writes LifeState). Deferred via Commands; the next
    // tick_bleed round reads the ganger un-marked and skips it.
    commands.entity(target_entity).remove::<BleedingOut>();
    // READ the flat cost to wire the leaf — debiting a Tu pool is E4, not here.
    Some(tuning.stabilize_tu)
}
