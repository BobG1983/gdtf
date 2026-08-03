//! Prefab authoring: [`PrefabSpec`], UUID-keyed level fragment.
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::{SpawnRole, TerrainPlacementEntry};
use crate::level::{GridSize, ThemeUuid};

/// Serde default helper (Fill) rather than `SpawnRole::default`.
const fn default_role() -> SpawnRole {
    SpawnRole::Fill
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct PrefabSpec {
    pub theme: ThemeUuid,
    pub size:  GridSize,
    /// Role this fragment plays when a level is assembled. Defaults to fill.
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
