use bevy::prelude::Deref;

use super::reach::{Actor, DownedTarget, is_8_adjacent};
use crate::{
    ganger::LifeState,
    tuning::{CombatTuning, ExecuteTu},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanExecute(bool);

impl CanExecute {
        #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

#[must_use]
pub fn can_execute(actor: &Actor, target: &DownedTarget) -> CanExecute {
    CanExecute::new(
        *is_8_adjacent(actor.pos, target.pos)
            && actor.life == LifeState::Alive
            && target.life == LifeState::Downed
            && actor.faction != target.faction,
    )
}

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
