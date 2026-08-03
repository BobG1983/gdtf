//! TU for stance change and facing turn.

use bevy::prelude::Deref;
use serde::Deserialize;

/// TU to change stance.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct StanceChangeTu(u8);

impl StanceChangeTu {
    /// Wrap a cost.
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for StanceChangeTu {
    fn default() -> Self {
        Self(8)
    }
}

/// TU to change facing.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct TurnTu(u8);

impl TurnTu {
    /// Wrap a cost.
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for TurnTu {
    fn default() -> Self {
        Self(1)
    }
}
