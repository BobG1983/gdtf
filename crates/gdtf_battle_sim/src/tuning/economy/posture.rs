use bevy::prelude::Deref;
use serde::Deserialize;

/// this value (the drop equals it), never the magnitude. `#[serde(transparent)]` lets
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct StanceChangeTu(u8);

impl StanceChangeTu {
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

/// magnitude. `#[serde(transparent)]` lets it parse a bare RON scalar; private inner +
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct TurnTu(u8);

impl TurnTu {
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
