use std::fmt::Display;

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// `#[serde(transparent)]` so it parses a bare RON scalar (the
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TuCost(u16);

impl TuCost {
        #[must_use]
    pub const fn new(tu: u16) -> Self {
        Self(tu)
    }
}

/// + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Strikes(u16);

impl Strikes {
        #[must_use]
    pub const fn new(strikes: u16) -> Self {
        Self(strikes)
    }
}

/// `#[derive(Component)]` — the [`FightMode`] selector that holds the specs is the
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FightModeSpec {
            pub kind:    FightModeKind,
        pub tu_cost: TuCost,
        pub strikes: Strikes,
}

impl FightModeSpec {
        #[must_use]
    pub const fn new(kind: FightModeKind, tu_cost: TuCost, strikes: Strikes) -> Self {
        Self {
            kind,
            tu_cost,
            strikes,
        }
    }
}

/// `.get()`); `#[serde(transparent)]` so it deserializes from a **bare RON list** of
/// a `Vec`). A `#[derive(Component)]` — the selector lives as a sibling component on
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct FightMode(Vec<FightModeSpec>);

impl FightMode {
        #[must_use]
    pub const fn new(modes: Vec<FightModeSpec>) -> Self {
        Self(modes)
    }

                            #[must_use]
    pub fn primary(&self) -> FightModeSpec {
        if let Some(first) = self.0.first() {
            *first
        } else {
            FightModeSpec::new(FightModeKind::Swing, TuCost::new(0), Strikes::new(1))
        }
    }
}
