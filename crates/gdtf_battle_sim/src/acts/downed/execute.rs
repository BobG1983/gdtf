//! Execute a downed enemy.

use bevy::prelude::Deref;

use super::reach::{Actor, DownedTarget, is_8_adjacent};
use crate::{
    ganger::{LifeState, Tu},
    tuning::{CombatTuning, ExecuteTu},
};

/// Whether execute is allowed for this pair.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanExecute(bool);

impl CanExecute {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

/// TU charged for executing one downed ganger.
#[must_use]
pub fn execute_tu_cost(tuning: &CombatTuning) -> Tu {
    Tu::new(*tuning.execute_tu)
}

/// Alive actor, adjacent, enemy, and target is downed.
#[must_use]
pub fn can_execute(actor: &Actor, target: &DownedTarget) -> CanExecute {
    CanExecute::new(
        *is_8_adjacent(actor.pos, target.pos)
            && actor.life == LifeState::Alive
            && target.life == LifeState::Downed
            && actor.faction != target.faction,
    )
}

/// Kill the target if allowed; returns the TU cost when applied.
pub fn execute_downed(
    actor: &Actor,
    target: &DownedTarget,
    target_life: &mut LifeState,
    tuning: &CombatTuning,
) -> Option<ExecuteTu> {
    if !*can_execute(actor, target) {
        return None;
    }
    *target_life = LifeState::Dead;
    Some(tuning.execute_tu)
}
