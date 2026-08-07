//! Firing-arc gate: fire in arc, turn then fire, or reject.

use bevy::prelude::Deref;

use crate::{
    combatants::firing_arc::target_in_arc,
    ganger::{Direction, Tu},
    metric::Cell,
    posture::turn_tu_cost,
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
        facing:    Direction,
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
    let Some((target_facing, turn_cost)) = turn_into_arc(facing, actor_cell, target_cell, tuning)
    else {
        return FireArcDecision::FireInArc;
    };
    if *tu >= *turn_then_fire_cost(turn_cost, fire_cost) {
        FireArcDecision::TurnThenFire {
            facing: target_facing,
            turn_cost,
        }
    } else {
        FireArcDecision::Reject
    }
}

/// The TU one shot charges from this facing: the shot, plus the turn it needs to face the target.
#[must_use]
pub fn fire_arc_tu_cost(
    facing: Direction,
    actor_cell: Cell,
    target_cell: Cell,
    fire_cost: Tu,
    tuning: &CombatTuning,
) -> Tu {
    match turn_into_arc(facing, actor_cell, target_cell, tuning) {
        Some((_, turn_cost)) => turn_then_fire_cost(turn_cost, fire_cost),
        None => fire_cost,
    }
}

// The facing the shot needs and what turning to it costs, or nothing when the target is in arc.
fn turn_into_arc(
    facing: Direction,
    actor_cell: Cell,
    target_cell: Cell,
    tuning: &CombatTuning,
) -> Option<(Direction, Tu)> {
    if *target_in_arc(facing, actor_cell, target_cell, &tuning.firing_arc) {
        return None;
    }
    let target_facing = Direction::from_cells(actor_cell, target_cell)?;
    Some((
        target_facing,
        turn_tu_cost(facing.steps_to(target_facing), &tuning.turn_tu),
    ))
}

fn turn_then_fire_cost(turn_cost: Tu, fire_cost: Tu) -> Tu {
    Tu::new((*turn_cost).saturating_add(*fire_cost))
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
