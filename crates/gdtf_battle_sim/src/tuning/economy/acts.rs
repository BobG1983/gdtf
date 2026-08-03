//! TU costs for shove, doors, emplacements, throws.

use bevy::prelude::Deref;
use serde::Deserialize;

/// TU for a deliberate shove.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ShoveTu(u8);

impl ShoveTu {
    /// Wrap a cost.
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

/// TU to open a door.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct OpenDoorTu(u8);

impl OpenDoorTu {
    /// Wrap a cost.
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

/// TU to enter an emplacement.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct EnterEmplacementTu(u8);

impl EnterEmplacementTu {
    /// Wrap a cost.
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

/// TU to exit an emplacement.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ExitEmplacementTu(u8);

impl ExitEmplacementTu {
    /// Wrap a cost.
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

/// TU to throw a grenade.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ThrowTu(u8);

impl ThrowTu {
    /// Wrap a cost.
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
