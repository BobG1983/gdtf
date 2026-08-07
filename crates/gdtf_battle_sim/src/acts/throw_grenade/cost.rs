//! TU cost and legality queries for throwing an arc weapon.

use bevy::prelude::Deref;

use crate::{
    ganger::Tu, magazine::Magazine, tu::can_spend_tu, tuning::CombatTuning, weapon::TrajectoryStyle,
};

/// Whether the thrower may throw this weapon right now.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanThrowGrenade(bool);

impl CanThrowGrenade {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

/// TU charged for one throw.
#[must_use]
pub fn throw_grenade_tu_cost(tuning: &CombatTuning) -> Tu {
    Tu::new(*tuning.throw_tu)
}

/// Arc weapon, a round left to throw, and the pool covers the cost.
#[must_use]
pub fn can_throw_grenade(
    trajectory: TrajectoryStyle,
    magazine: &Magazine,
    tu: &Tu,
    tuning: &CombatTuning,
) -> CanThrowGrenade {
    CanThrowGrenade::new(
        *trajectory.is_arc()
            && !*magazine.is_empty()
            && *can_spend_tu(tu, throw_grenade_tu_cost(tuning)),
    )
}
