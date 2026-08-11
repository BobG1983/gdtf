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

/// TU charged for taking or dropping aim. Aiming is free.
#[must_use]
pub const fn set_aiming_tu_cost() -> Tu {
    Tu::new(0)
}

/// TU charged for one stance change.
#[must_use]
pub fn stance_tu_cost(cost: &StanceChangeTu) -> Tu {
    Tu::new(**cost)
}

/// Why the sim will not change stance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StanceRefusal {
    /// The actor already holds that stance.
    AlreadyHeld,
    /// The actor cannot afford the change.
    Unaffordable,
}

/// The first rule this stance change breaks, or nothing when it is allowed.
#[must_use]
pub fn stance_refusal(
    stance: &Stance,
    to: StanceKind,
    tu: &Tu,
    cost: &StanceChangeTu,
) -> Option<StanceRefusal> {
    if **stance == to {
        return Some(StanceRefusal::AlreadyHeld);
    }
    (!*can_spend_tu(tu, stance_tu_cost(cost))).then_some(StanceRefusal::Unaffordable)
}

/// Stance differs from the one held and the pool covers the change.
#[must_use]
pub fn can_set_stance(
    stance: &Stance,
    to: StanceKind,
    tu: &Tu,
    cost: &StanceChangeTu,
) -> CanSetStance {
    CanSetStance::new(stance_refusal(stance, to, tu, cost).is_none())
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

/// Why the sim will not turn the actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FacingRefusal {
    /// The actor already faces that way.
    AlreadyFacing,
    /// The pool cannot pay for even one ring step.
    Unaffordable,
}

/// The first rule this turn breaks, or nothing when it is allowed.
#[must_use]
pub fn facing_refusal(
    from: Direction,
    to: Direction,
    tu: &Tu,
    cost: &TurnTu,
) -> Option<FacingRefusal> {
    if *from.steps_to(to) == 0 {
        return Some(FacingRefusal::AlreadyFacing);
    }
    (*afforded_turn_steps(from, to, tu, cost) == 0).then_some(FacingRefusal::Unaffordable)
}

/// The turn changes facing and the pool covers at least one ring step.
#[must_use]
pub fn can_set_facing(from: Direction, to: Direction, tu: &Tu, cost: &TurnTu) -> CanSetFacing {
    CanSetFacing::new(facing_refusal(from, to, tu, cost).is_none())
}
