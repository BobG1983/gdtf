//! An authored prefab's spawn role and its painted-cell count on the wire.

use bevy::prelude::Deref;
use gdtf_battle_sim::level::SpawnRole;
use serde::{Deserialize, Serialize};

/// The part a prefab plays when a level is assembled, mirroring the sim's own role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum SpawnRoleNet {
    /// Player-side starting area.
    Player,
    /// Enemy-side starting area.
    Enemy,
    /// Fill / neutral terrain.
    Fill,
}

impl SpawnRoleNet {
    /// Every role a prefab key can carry, for a case that walks them all.
    #[cfg(test)]
    pub(in crate::mcp) const ALL: [Self; 3] = [Self::Player, Self::Enemy, Self::Fill];

    /// Read a client's role back as the sim's own.
    pub(in crate::mcp) const fn to_role(self) -> SpawnRole {
        match self {
            Self::Player => SpawnRole::Player,
            Self::Enemy => SpawnRole::Enemy,
            Self::Fill => SpawnRole::Fill,
        }
    }
}

/// How many cells a loaded prefab painted onto the canvas.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct PrefabPlacementCountNet(usize);

impl PrefabPlacementCountNet {
    /// Wrap a painted-cell count.
    #[must_use]
    pub(in crate::mcp) const fn new(cells: usize) -> Self {
        Self(cells)
    }
}
