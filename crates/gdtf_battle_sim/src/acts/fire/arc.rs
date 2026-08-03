//! Firing-arc gate: fire in arc, turn then fire, or reject.

use bevy::prelude::Deref;

use crate::{
    combatants::firing_arc::target_in_arc,
    ganger::{Direction, Tu},
    metric::Cell,
    tuning::CombatTuning,
};

/// Whether the shooter can engage the target this turn (with optional turn cost).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanEngage(bool);

impl CanEngage {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(engageable: bool) -> Self {
        Self(engageable)
    }
}

/// How to handle a fire request relative to facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireArcDecision {
    /// Already facing into arc.
    FireInArc,
    /// Must spend turn TU then fire.
    TurnThenFire {
        /// Facing to adopt.
        facing: Direction,
        /// Turn cost.
        turn_cost: Tu,
    },
    /// Not enough TU to turn and fire.
    Reject,
}

/// Decide whether the shot is in arc or needs a facing change.
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
    let Some(target_facing) = Direction::from_cells(actor_cell, target_cell) else {
        return FireArcDecision::FireInArc;
    };
    let turn_cost = Tu::new(
        facing
            .steps_to(target_facing)
            .saturating_mul(*tuning.turn_tu),
    );
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

/// True unless the arc decision is [`FireArcDecision::Reject`].
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
