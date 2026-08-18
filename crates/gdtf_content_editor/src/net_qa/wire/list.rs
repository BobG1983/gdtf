//! Which list a write names, what it does to it, and how its members read back.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::facing::TerrainFacingNet;

/// One list-valued field of the active mode's draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorListNet {
    /// The Terrain draft's emplacement entry sides.
    EntrySides,
}

/// What a write does to the named list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorListOpNet {
    /// Add the member if absent, remove it if present.
    Toggle(TerrainFacingNet),
}

/// One member of the list a reply reads back.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorListMemberNet(TerrainFacingNet);

impl EditorListMemberNet {
    /// Wrap a member of the entry-sides list.
    #[must_use]
    pub(in crate::net_qa) const fn from_facing(facing: TerrainFacingNet) -> Self {
        Self(facing)
    }
}
