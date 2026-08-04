//! Wound costs, bleed rate, stabilize/execute TU.

use bevy::prelude::Deref;
use serde::Deserialize;

/// Wound pool cost for one severity tier.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct WoundCost(u8);

impl WoundCost {
    /// Wrap a cost.
    #[must_use]
    pub const fn new(cost: u8) -> Self {
        Self(cost)
    }
}

/// Bleed damage per tick.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct BleedRate(u8);

impl BleedRate {
    /// Wrap a rate.
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

/// TU to stabilize a downed ganger.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct StabilizeTu(u8);

impl StabilizeTu {
    /// Wrap a TU cost.
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

/// TU to execute a downed ganger.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ExecuteTu(u8);

impl ExecuteTu {
    /// Wrap a TU cost.
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

/// Costs for minor / major / critical wounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct WoundCosts {
    /// Minor.
    pub minor:    WoundCost,
    /// Major.
    pub major:    WoundCost,
    /// Critical.
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
