use bevy::prelude::Deref;

use crate::{
    combatants::firing_arc::target_in_arc,
    ganger::{Direction, Tu},
    metric::Cell,
    tuning::CombatTuning,
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanEngage(bool);

impl CanEngage {
        #[must_use]
    pub const fn new(engageable: bool) -> Self {
        Self(engageable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireArcDecision {
                FireInArc,
        TurnThenFire {
                facing:    Direction,
                turn_cost: Tu,
    },
            Reject,
}

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
