//! Ganger vitals on the wire.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// Remaining time units.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TuNet(u8);

impl TuNet {
    /// Build from a raw TU value.
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

/// Time units at the start of a turn.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TuMaxNet(u8);

impl TuMaxNet {
    /// Build from a raw max TU value.
    #[must_use]
    pub const fn new(tu_max: u8) -> Self {
        Self(tu_max)
    }
}

/// Current hit points.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HpNet(u16);

impl HpNet {
    /// Build from a raw HP value.
    #[must_use]
    pub const fn new(hp: u16) -> Self {
        Self(hp)
    }
}

/// Hit points at full health.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HpMaxNet(u16);

impl HpMaxNet {
    /// Build from a raw max HP value.
    #[must_use]
    pub const fn new(hp_max: u16) -> Self {
        Self(hp_max)
    }
}

/// Wounds still standing between a ganger and going down.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WoundsNet(u8);

impl WoundsNet {
    /// Build from a raw wound count.
    #[must_use]
    pub const fn new(wounds: u8) -> Self {
        Self(wounds)
    }
}

/// Wound count at full health.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WoundsMaxNet(u8);

impl WoundsMaxNet {
    /// Build from a raw max wound count.
    #[must_use]
    pub const fn new(wounds_max: u8) -> Self {
        Self(wounds_max)
    }
}
