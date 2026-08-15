//! The painted-cell coordinate an illegal-cell save fault names.

use bevy::prelude::Deref;
use gdtf_battle_sim::metric::CellLevel;
use serde::{Deserialize, Serialize};

/// Cell X coordinate.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorCellXNet(i32);

impl EditorCellXNet {
    /// Build from a raw X value.
    #[must_use]
    pub(in crate::net_qa) const fn new(x: i32) -> Self {
        Self(x)
    }
}

/// Cell Y coordinate.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorCellYNet(i32);

impl EditorCellYNet {
    /// Build from a raw Y value.
    #[must_use]
    pub(in crate::net_qa) const fn new(y: i32) -> Self {
        Self(y)
    }
}

/// Storey index.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorLevelNet(u8);

impl EditorLevelNet {
    /// Build from a storey index.
    #[must_use]
    pub(in crate::net_qa) const fn new(storey: u8) -> Self {
        Self(storey)
    }
}

/// A painted cell and its storey on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorCellLevelNet {
    /// X coordinate.
    x:     EditorCellXNet,
    /// Y coordinate.
    y:     EditorCellYNet,
    /// Storey.
    level: EditorLevelNet,
}

impl EditorCellLevelNet {
    /// Mirror the editor's own painted-cell key.
    #[must_use]
    pub(in crate::net_qa) fn from_slot(slot: CellLevel) -> Self {
        let (cell, level) = slot.split();
        Self {
            x:     EditorCellXNet::new(cell.x),
            y:     EditorCellYNet::new(cell.y),
            level: EditorLevelNet::new(*level),
        }
    }
}
