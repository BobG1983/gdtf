//! The **prefab authoring struct** — [`PrefabSpec`], the UUID-keyed level-fragment
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::{SpawnRole, TerrainPlacementEntry};
use crate::level::{GridSize, ThemeUuid};

/// `#[serde(default = "...")]` helper rather than `SpawnRole::default`.
const fn default_role() -> SpawnRole {
    SpawnRole::Fill
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct PrefabSpec {
        pub theme:      ThemeUuid,
                pub size:       GridSize,
    /// The [`SpawnRole`] this fragment plays in an assembled level. `#[serde(default =
            #[serde(default = "default_role")]
    pub role:       SpawnRole,
                pub placements: Vec<TerrainPlacementEntry>,
}

impl PrefabSpec {
        #[must_use]
    pub const fn new(
        theme: ThemeUuid,
        size: GridSize,
        role: SpawnRole,
        placements: Vec<TerrainPlacementEntry>,
    ) -> Self {
        Self {
            theme,
            size,
            role,
            placements,
        }
    }
}
