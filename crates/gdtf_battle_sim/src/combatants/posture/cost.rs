//! TU cost and legality queries for stance and facing changes.

use bevy::prelude::Deref;

use crate::{
    ganger::{Direction, RingSteps, Stance, StanceKind, Tu},
    tu::can_spend_tu,
    tuning::{StanceChangeTu, TurnTu},
};

/// Whether a stance change is legal and affordable.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanSetStance(bool);

impl CanSetStance {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

/// Whether a facing change turns at least one ring step.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanSetFacing(bool);

impl CanSetFacing {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

/// TU charged for one stance change.
#[must_use]
pub fn stance_tu_cost(cost: &StanceChangeTu) -> Tu {
    Tu::new(**cost)
}

/// Stance differs from the one held and the pool covers the change.
#[must_use]
pub fn can_set_stance(
    stance: &Stance,
    to: StanceKind,
    tu: &Tu,
    cost: &StanceChangeTu,
) -> CanSetStance {
    CanSetStance::new(**stance != to && *can_spend_tu(tu, stance_tu_cost(cost)))
}

/// TU charged for turning `steps` ring steps.
#[must_use]
pub fn turn_tu_cost(steps: RingSteps, cost: &TurnTu) -> Tu {
    Tu::new((*steps).saturating_mul(**cost))
}

/// Ring steps of a short-way turn the pool can pay for.
#[must_use]
pub fn afforded_turn_steps(from: Direction, to: Direction, tu: &Tu, cost: &TurnTu) -> RingSteps {
    let total = *from.steps_to(to);
    let pool_steps = (**tu).checked_div(**cost).unwrap_or(total);
    RingSteps::new(pool_steps.min(total))
}

/// TU a turn from `from` to `to` charges once the pool limits it.
#[must_use]
pub fn afforded_turn_tu_cost(from: Direction, to: Direction, tu: &Tu, cost: &TurnTu) -> Tu {
    turn_tu_cost(afforded_turn_steps(from, to, tu, cost), cost)
}

/// The turn changes facing and the pool covers at least one ring step.
#[must_use]
pub fn can_set_facing(from: Direction, to: Direction, tu: &Tu, cost: &TurnTu) -> CanSetFacing {
    CanSetFacing::new(*afforded_turn_steps(from, to, tu, cost) > 0)
}
