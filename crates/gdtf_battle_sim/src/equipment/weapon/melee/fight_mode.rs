//! Melee strike modes (swing / thrust).

use std::fmt::Display;

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// TU cost for one melee mode.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TuCost(u16);

impl TuCost {
    /// Wrap a cost.
    #[must_use]
    pub const fn new(tu: u16) -> Self {
        Self(tu)
    }
}

/// Number of strikes in the mode.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Strikes(u16);

impl Strikes {
    /// Wrap a strike count.
    #[must_use]
    pub const fn new(strikes: u16) -> Self {
        Self(strikes)
    }
}

/// Swing or thrust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FightModeKind {
    Swing,
    Thrust,
}

impl Display for FightModeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Swing => "swing",
            Self::Thrust => "thrust",
        };
        f.write_str(label)
    }
}

/// One melee mode entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FightModeSpec {
    /// Kind.
    pub kind: FightModeKind,
    /// TU cost.
    pub tu_cost: TuCost,
    /// Strikes.
    pub strikes: Strikes,
}

impl FightModeSpec {
    /// Build a mode.
    #[must_use]
    pub const fn new(kind: FightModeKind, tu_cost: TuCost, strikes: Strikes) -> Self {
        Self {
            kind,
            tu_cost,
            strikes,
        }
    }
}

/// List of fight modes on a melee weapon.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FightMode(Vec<FightModeSpec>);

impl FightMode {
    /// Wrap a list of modes.
    #[must_use]
    pub const fn new(modes: Vec<FightModeSpec>) -> Self {
        Self(modes)
    }

    /// First mode, or a safe swing default.
    #[must_use]
    pub fn primary(&self) -> FightModeSpec {
        if let Some(first) = self.0.first() {
            *first
        } else {
            FightModeSpec::new(FightModeKind::Swing, TuCost::new(0), Strikes::new(1))
        }
    }
}
