//! The pure GTW-242 firing-arc gate — the arc verdict ([`FireArcDecision`]), its
//! decider ([`decide_fire_arc`]), and the AI-shared [`can_engage`] boolean wrapper.

use bevy::prelude::Deref;

use crate::{
    combatants::firing_arc::target_in_arc,
    ganger::{Direction, Tu},
    metric::Cell,
    tuning::CombatTuning,
};

/// Whether a shooter can **engage** a target under the firing-arc gate — the
/// [`can_engage`] verdict (`true` iff the arc verdict is not a reject; GTW-70).
///
/// The AI's turn-termination gate keys off this: an act is emitted ONLY when engagement
/// is legal, so the dispatcher can never silently reject a shot the AI keeps re-emitting.
/// A distinct engagement-permission predicate, not a bare `bool`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanEngage(bool);

impl CanEngage {
    /// Build the engagement verdict from the computed arc gate.
    #[must_use]
    pub const fn new(engageable: bool) -> Self {
        Self(engageable)
    }
}

/// The firing-arc gate's decision for ONE [`FireRequested`](crate::acts::request::FireRequested) — computed from the read state
/// BEFORE any mutation, so the spend/turn/shot is atomic-by-construction (GTW-242).
///
/// The arc rule (`docs/combat/resolution.md` §1 / the targeting section, USER ruling
/// 2026-06-16): an in-arc target fires directly; an out-of-arc target fires ONLY when the
/// shooter affords BOTH the turn-into-arc AND the shot — else the shot is REJECTED (no TU
/// spent, no facing change, no shot). This enum carries the pre-computed verdict so the
/// dispatch performs exactly the mutations the verdict allows.
///
/// GTW-70: made `pub` (with its decider [`decide_fire_arc`] and the [`can_engage`]
/// boolean wrapper) so the enemy-AI engagement gate and [`dispatch_fire`](super::dispatch::dispatch_fire) share the ONE
/// arc verdict — neither re-derives it. The AI consults [`can_engage`] (the `¬Reject`
/// boolean) to decide whether a target is shootable; the dispatcher matches the full enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireArcDecision {
    /// In-arc — fire directly (no turn). [`fire`](crate::fire::fire)'s own [`crate::magazine::can_fire`] gate
    /// handles fire-TU affordability (an in-arc shot the shooter cannot afford resolves to
    /// nothing inside [`fire`](crate::fire::fire), mutating nothing).
    FireInArc,
    /// Out-of-arc AND affordable — turn to `facing` (spending `turn_cost`) THEN fire.
    TurnThenFire {
        /// The compass facing toward the target ([`Direction::from_cells`]).
        facing:    Direction,
        /// The full turn-into-arc TU cost (`steps_to × turn_tu`), spent before the shot.
        turn_cost: Tu,
    },
    /// Out-of-arc and NOT affordable (cannot pay turn + fire) — REJECT: no spend, no turn,
    /// no shot.
    Reject,
}

/// Decide the firing-arc verdict for a shot from `actor_cell` (facing `facing`, current
/// pool `tu`) at `target_cell`, given the fire-TU cost and the [`CombatTuning`] (firing arc
/// + per-45°-step turn TU) — the pure GTW-242 gate, computed before any world mutation.
///
/// In-arc ([`target_in_arc`]) ⇒ [`FireArcDecision::FireInArc`]. Out-of-arc ⇒ compute the
/// target facing ([`Direction::from_cells`] — a co-located target is never out-of-arc, so
/// this is `Some`; a defensive `None` falls back to a direct shot) and the full turn cost
/// (`steps_to × turn_tu`, REUSING the landed [`Direction::steps_to`] + the
/// [`crate::tuning::TurnTu`] leaf — NOT re-derived); affordable iff `tu ≥ turn_cost +
/// fire_cost` ([`FireArcDecision::TurnThenFire`]), else [`FireArcDecision::Reject`]. The
/// `fire_cost`/`turn_cost` arithmetic is **saturating** on the `u8` pool (no overflow, no
/// panic). Pure, total.
///
/// GTW-70: `pub` so the enemy-AI engagement gate ([`crate::ai`]) and [`dispatch_fire`](super::dispatch::dispatch_fire)
/// share the ONE arc verdict. The AI usually calls the [`can_engage`] boolean wrapper; the
/// dispatcher matches the full [`FireArcDecision`] (it needs the turn cost / facing).
#[must_use]
pub fn decide_fire_arc(
    facing: Direction,
    actor_cell: Cell,
    target_cell: Cell,
    tu: Tu,
    fire_cost: Tu,
    tuning: &CombatTuning,
) -> FireArcDecision {
    if *target_in_arc(facing, actor_cell, target_cell, &tuning.firing_arc) {
        return FireArcDecision::FireInArc;
    }
    // Out-of-arc: the shot needs a turn-into-arc first. from_cells is Some here (a
    // co-located target is in-arc, handled above); a defensive None degrades to a direct
    // shot rather than panicking.
    let Some(target_facing) = Direction::from_cells(actor_cell, target_cell) else {
        return FireArcDecision::FireInArc;
    };
    // The FULL turn cost, REUSING the landed steps_to (short-way 45° count) × the TurnTu
    // leaf — NOT the partial set_facing path. steps_to ∈ 0..=4 and turn_tu is small, so
    // saturating_mul never wraps (and stays panic-free regardless).
    let turn_cost = Tu::new(
        facing
            .steps_to(target_facing)
            .saturating_mul(*tuning.turn_tu),
    );
    // The COMBINED gate (the user's stricter ruling): afford BOTH turn AND fire, or reject.
    // Saturating add keeps the sum panic-free even if the costs were pathologically large.
    let combined = (*turn_cost).saturating_add(*fire_cost);
    if *tu >= combined {
        FireArcDecision::TurnThenFire {
            facing: target_facing,
            turn_cost,
        }
    } else {
        FireArcDecision::Reject
    }
}

/// Whether a shooter at `actor_cell` (facing `facing`, pool `tu`) can **engage** a target
/// at `target_cell` under the GTW-242 firing-arc gate — `true` iff the arc verdict is NOT
/// [`FireArcDecision::Reject`] (GTW-70).
///
/// The boolean shape of [`decide_fire_arc`] the enemy-AI engagement gate ([`crate::ai`])
/// shares with [`dispatch_fire`](super::dispatch::dispatch_fire): an in-arc shot ([`FireArcDecision::FireInArc`]) and an
/// out-of-arc-but-affordable shot ([`FireArcDecision::TurnThenFire`]) both return `true`;
/// only the unaffordable out-of-arc reject returns `false`. Load-bearing for the AI's
/// turn-termination guarantee: the AI emits a `FireRequested` ONLY when this is `true`, so
/// the dispatcher can never silently reject the shot (spend no TU) and let the AI re-emit
/// it forever. Pure, total — a thin `matches!` over the SAME verdict the dispatcher runs.
#[must_use]
pub fn can_engage(
    facing: Direction,
    actor_cell: Cell,
    target_cell: Cell,
    tu: Tu,
    fire_cost: Tu,
    tuning: &CombatTuning,
) -> CanEngage {
    CanEngage::new(!matches!(
        decide_fire_arc(facing, actor_cell, target_cell, tu, fire_cost, tuning),
        FireArcDecision::Reject
    ))
}
