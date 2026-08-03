use bevy::prelude::Deref;
use serde::Deserialize;

/// value (the drop equals it), never the magnitude. `#[serde(transparent)]` lets it parse a
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ShoveTu(u8);

impl ShoveTu {
                                #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for ShoveTu {
    fn default() -> Self {
        Self(6)
    }
}

/// never the magnitude. `#[serde(transparent)]` lets it parse a bare RON scalar; private inner
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct OpenDoorTu(u8);

impl OpenDoorTu {
                            #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for OpenDoorTu {
    fn default() -> Self {
        Self(4)
    }
}

/// this value (the drop equals it), never the magnitude. `#[serde(transparent)]` lets it parse
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct EnterEmplacementTu(u8);

impl EnterEmplacementTu {
                            #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for EnterEmplacementTu {
    fn default() -> Self {
        Self(6)
    }
}

/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ExitEmplacementTu(u8);

impl ExitEmplacementTu {
                            #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for ExitEmplacementTu {
    fn default() -> Self {
        Self(4)
    }
}

/// (the drop equals it), never the magnitude. `#[serde(transparent)]` lets it parse a bare RON
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ThrowTu(u8);

impl ThrowTu {
                            #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for ThrowTu {
    fn default() -> Self {
        Self(6)
    }
}
