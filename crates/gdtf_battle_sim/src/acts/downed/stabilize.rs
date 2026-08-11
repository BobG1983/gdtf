//! Stabilize a downed ally (stop bleed).

use bevy::prelude::{Commands, Deref, Entity};

use super::reach::{Actor, DownedTarget, is_8_adjacent};
use crate::{
    effects::bleed::BleedingOut,
    ganger::{LifeState, Tu},
    tu::can_spend_tu,
    tuning::{CombatTuning, StabilizeTu},
};

/// Whether stabilize is allowed for this pair.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanStabilize(bool);

impl CanStabilize {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

/// TU charged for stabilizing one bleeding ganger.
#[must_use]
pub fn stabilize_tu_cost(tuning: &CombatTuning) -> Tu {
    Tu::new(*tuning.stabilize_tu)
}

/// Alive actor, adjacent, same faction, target downed and bleeding, pool covers the cost.
#[must_use]
pub fn can_stabilize(
    actor: &Actor,
    target: &DownedTarget,
    tu: &Tu,
    tuning: &CombatTuning,
) -> CanStabilize {
    CanStabilize::new(
        *is_8_adjacent(actor.pos, target.pos)
            && actor.life == LifeState::Alive
            && target.life == LifeState::Downed
            && actor.faction == target.faction
            && target.bleeding_out.is_some()
            && *can_spend_tu(tu, stabilize_tu_cost(tuning)),
    )
}

/// Remove bleed if allowed; returns the TU cost when applied.
pub fn stabilize_downed(
    actor: &Actor,
    target: &DownedTarget,
    target_entity: Entity,
    commands: &mut Commands,
    tu: &Tu,
    tuning: &CombatTuning,
) -> Option<StabilizeTu> {
    if !*can_stabilize(actor, target, tu, tuning) {
        return None;
    }
    commands.entity(target_entity).remove::<BleedingOut>();
    Some(tuning.stabilize_tu)
}
