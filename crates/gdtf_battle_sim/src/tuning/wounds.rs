use bevy::prelude::Deref;
use serde::Deserialize;

/// Minor < Major < Critical). `#[serde(transparent)]` lets it parse a bare RON
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct WoundCost(u8);

impl WoundCost {
                            #[must_use]
    pub const fn new(cost: u8) -> Self {
        Self(cost)
    }
}

/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner +
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct BleedRate(u8);

impl BleedRate {
                                #[must_use]
    pub const fn new(rate: u8) -> Self {
        Self(rate)
    }
}

impl Default for BleedRate {
    fn default() -> Self {
        Self(1)
    }
}

/// value-agnostic. `#[serde(transparent)]` lets it parse a bare RON scalar; private
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct StabilizeTu(u8);

impl StabilizeTu {
                                #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for StabilizeTu {
    fn default() -> Self {
        Self(4)
    }
}

/// value-agnostic. `#[serde(transparent)]` lets it parse a bare RON scalar; private
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ExecuteTu(u8);

impl ExecuteTu {
                                #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for ExecuteTu {
    fn default() -> Self {
        Self(6)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct WoundCosts {
        pub minor:    WoundCost,
        pub major:    WoundCost,
        pub critical: WoundCost,
}

impl Default for WoundCosts {
    fn default() -> Self {
        Self {
            minor:    WoundCost(1),
            major:    WoundCost(2),
            critical: WoundCost(3),
        }
    }
}
