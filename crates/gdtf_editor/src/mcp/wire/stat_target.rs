//! The stat an injury's Modify effect names, on the wire.

use gdtf_battle_sim::injuries::StatTarget;
use serde::{Deserialize, Serialize};

/// Which stat a Modify effect moves, mirroring the sim's own target with no wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum StatTargetNet {
    /// Movement speed attribute.
    Speed,
    /// Aim attribute.
    Aim,
    /// Strength attribute.
    Strength,
    /// Toughness attribute.
    Toughness,
    /// Reflexes attribute.
    Reflexes,
    /// Cool attribute.
    Cool,
    /// Grit attribute.
    Grit,
    /// Luck attribute.
    Luck,
    /// Derived shooting skill.
    Shooting,
    /// Derived fight skill.
    Fight,
    /// Derived reactions skill.
    Reactions,
    /// Derived morale.
    Morale,
    /// Time-unit pool.
    Tu,
    /// Hit points.
    Hp,
    /// Wound count.
    Wounds,
    /// Bottle threshold.
    Bottle,
}

impl StatTargetNet {
    /// Mirror the sim's own stat target.
    pub(in crate::mcp) const fn from_target(target: StatTarget) -> Self {
        match target {
            StatTarget::Speed => Self::Speed,
            StatTarget::Aim => Self::Aim,
            StatTarget::Strength => Self::Strength,
            StatTarget::Toughness => Self::Toughness,
            StatTarget::Reflexes => Self::Reflexes,
            StatTarget::Cool => Self::Cool,
            StatTarget::Grit => Self::Grit,
            StatTarget::Luck => Self::Luck,
            StatTarget::Shooting => Self::Shooting,
            StatTarget::Fight => Self::Fight,
            StatTarget::Reactions => Self::Reactions,
            StatTarget::Morale => Self::Morale,
            StatTarget::Tu => Self::Tu,
            StatTarget::Hp => Self::Hp,
            StatTarget::Wounds => Self::Wounds,
            StatTarget::Bottle => Self::Bottle,
        }
    }

    /// Read a client's stat target back as the sim's own.
    pub(in crate::mcp) const fn to_target(self) -> StatTarget {
        match self {
            Self::Speed => StatTarget::Speed,
            Self::Aim => StatTarget::Aim,
            Self::Strength => StatTarget::Strength,
            Self::Toughness => StatTarget::Toughness,
            Self::Reflexes => StatTarget::Reflexes,
            Self::Cool => StatTarget::Cool,
            Self::Grit => StatTarget::Grit,
            Self::Luck => StatTarget::Luck,
            Self::Shooting => StatTarget::Shooting,
            Self::Fight => StatTarget::Fight,
            Self::Reactions => StatTarget::Reactions,
            Self::Morale => StatTarget::Morale,
            Self::Tu => StatTarget::Tu,
            Self::Hp => StatTarget::Hp,
            Self::Wounds => StatTarget::Wounds,
            Self::Bottle => StatTarget::Bottle,
        }
    }
}
