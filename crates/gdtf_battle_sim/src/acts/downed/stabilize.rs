use bevy::prelude::{Commands, Deref, Entity};

use super::reach::{Actor, DownedTarget, is_8_adjacent};
use crate::{
    effects::bleed::BleedingOut,
    ganger::LifeState,
    tuning::{CombatTuning, StabilizeTu},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanStabilize(bool);

impl CanStabilize {
        #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

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
    commands.entity(target_entity).remove::<BleedingOut>();
    Some(tuning.stabilize_tu)
}
