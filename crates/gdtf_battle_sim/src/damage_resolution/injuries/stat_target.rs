//! Which combatant stat an injury effect can modify.

use serde::{Deserialize, Serialize};

/// Stats that injury effects can touch.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Deserialize, Serialize)]
pub enum StatTarget {
    Speed,
    Aim,
    Strength,
    Toughness,
    Reflexes,
    Cool,
    Grit,
    Luck,
    Shooting,
    Fight,
    Reactions,
    Morale,
    Tu,
    Hp,
    Wounds,
    Bottle,
}

/// Whether a stat is a base attribute or a derived value.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum StatKind {
    /// Primary attribute.
    Attribute,
    /// Derived from attributes or other state.
    Derived,
}

impl StatTarget {
    /// All targets in index order.
    pub const ALL: [Self; 16] = [
        Self::Speed,
        Self::Aim,
        Self::Strength,
        Self::Toughness,
        Self::Reflexes,
        Self::Cool,
        Self::Grit,
        Self::Luck,
        Self::Shooting,
        Self::Fight,
        Self::Reactions,
        Self::Morale,
        Self::Tu,
        Self::Hp,
        Self::Wounds,
        Self::Bottle,
    ];

    /// Number of targets.
    pub const COUNT: usize = Self::ALL.len();

    /// Attribute vs derived.
    #[must_use]
    pub const fn kind(self) -> StatKind {
        match self {
            Self::Speed
            | Self::Aim
            | Self::Strength
            | Self::Toughness
            | Self::Reflexes
            | Self::Cool
            | Self::Grit
            | Self::Luck => StatKind::Attribute,
            Self::Shooting
            | Self::Fight
            | Self::Reactions
            | Self::Morale
            | Self::Tu
            | Self::Hp
            | Self::Wounds
            | Self::Bottle => StatKind::Derived,
        }
    }

    /// Dense index used by the delta ledger.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Speed => 0,
            Self::Aim => 1,
            Self::Strength => 2,
            Self::Toughness => 3,
            Self::Reflexes => 4,
            Self::Cool => 5,
            Self::Grit => 6,
            Self::Luck => 7,
            Self::Shooting => 8,
            Self::Fight => 9,
            Self::Reactions => 10,
            Self::Morale => 11,
            Self::Tu => 12,
            Self::Hp => 13,
            Self::Wounds => 14,
            Self::Bottle => 15,
        }
    }
}
