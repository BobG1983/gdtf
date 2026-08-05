//! Enemies, doors and cover inside the lit area, on the wire.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::{
    cell::CellLevelNet,
    inspect::CoverBlockNet,
    token::{DoorToken, GangerToken},
};

/// Whether a door stands open.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DoorOpenNet(bool);

impl DoorOpenNet {
    /// Build from an open flag.
    #[must_use]
    pub const fn new(open: bool) -> Self {
        Self(open)
    }
}

/// An enemy the squad can see, and where it stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibleGangerNet {
    /// Opaque ganger identity.
    pub token: GangerToken,
    /// Where it stands.
    pub at:    CellLevelNet,
}

impl VisibleGangerNet {
    /// Build from a token and a cell.
    #[must_use]
    pub const fn new(token: GangerToken, at: CellLevelNet) -> Self {
        Self { token, at }
    }
}

/// A door the squad can see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibleDoorNet {
    /// Opaque door identity.
    pub token: DoorToken,
    /// Where it stands.
    pub at:    CellLevelNet,
    /// Whether it stands open.
    pub open:  DoorOpenNet,
}

impl VisibleDoorNet {
    /// Build from a token, a cell and an open flag.
    #[must_use]
    pub const fn new(token: DoorToken, at: CellLevelNet, open: DoorOpenNet) -> Self {
        Self { token, at, open }
    }
}

/// A cover cell the squad can see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibleCoverNet {
    /// Where it stands.
    pub at:    CellLevelNet,
    /// What the inspect panel would show for it.
    pub cover: CoverBlockNet,
}

impl VisibleCoverNet {
    /// Build from a cell and a cover block.
    #[must_use]
    pub const fn new(at: CellLevelNet, cover: CoverBlockNet) -> Self {
        Self { at, cover }
    }
}
