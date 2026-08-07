//! TU cost and legality queries for a deliberate shove.

use bevy::prelude::Deref;

use crate::{
    acts::downed::is_8_adjacent,
    ganger::{Faction, LifeState, Position, Tu},
    tuning::CombatTuning,
};

/// Whether a deliberate shove is legal.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanShove(bool);

impl CanShove {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

/// Shover geometry and allegiance for the reach check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShoveActor {
    /// Position.
    pub position: Position,
    /// Faction.
    pub faction:  Faction,
}

/// Target geometry, allegiance, and life for the reach check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShoveTarget {
    /// Position.
    pub position: Position,
    /// Faction.
    pub faction:  Faction,
    /// Life state.
    pub life:     LifeState,
}

/// TU charged for one deliberate shove.
#[must_use]
pub fn shove_tu_cost(tuning: &CombatTuning) -> Tu {
    Tu::new(*tuning.shove_tu)
}

/// Adjacent, hostile, and the target is still active.
#[must_use]
pub fn can_shove(shover: &ShoveActor, target: &ShoveTarget) -> CanShove {
    CanShove::new(
        *is_8_adjacent(shover.position, target.position)
            && shover.faction != target.faction
            && *target.life.is_active(),
    )
}
