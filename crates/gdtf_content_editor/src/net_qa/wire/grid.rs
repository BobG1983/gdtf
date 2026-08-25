//! The prefab grid's extent on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::level::GridSize;
use serde::{Deserialize, Serialize};

/// Grid width in cells.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorGridWidthNet(u8);

impl EditorGridWidthNet {
    /// Wrap a width.
    #[must_use]
    pub(in crate::net_qa) const fn new(cells: u8) -> Self {
        Self(cells)
    }
}

/// Grid height in cells.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorGridHeightNet(u8);

impl EditorGridHeightNet {
    /// Wrap a height.
    #[must_use]
    pub(in crate::net_qa) const fn new(cells: u8) -> Self {
        Self(cells)
    }
}

/// Storey count the grid allows.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorGridLevelsNet(u8);

impl EditorGridLevelsNet {
    /// Wrap a storey count.
    #[must_use]
    pub(in crate::net_qa) const fn new(levels: u8) -> Self {
        Self(levels)
    }
}

/// The authoring session's grid extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorGridSizeNet {
    /// Cells across.
    width:  EditorGridWidthNet,
    /// Cells down.
    height: EditorGridHeightNet,
    /// Storeys.
    levels: EditorGridLevelsNet,
}

impl EditorGridSizeNet {
    /// Mirror the session's own validated grid size.
    #[must_use]
    pub(in crate::net_qa) fn from_size(size: GridSize) -> Self {
        Self {
            width:  EditorGridWidthNet::new(*size.width()),
            height: EditorGridHeightNet::new(*size.height()),
            levels: EditorGridLevelsNet::new(*size.levels()),
        }
    }
}
