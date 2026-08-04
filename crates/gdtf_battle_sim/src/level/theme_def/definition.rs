//! UUID-keyed theme definition.

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::ThemeUuid;
use crate::terrain::def::TerrainUuid;

/// Human-readable theme label.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct ThemeDisplayName(String);

impl ThemeDisplayName {
    /// Wrap a display name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// Theme definition keyed by UUID with default floor and terrain set.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct UuidThemeDef {
    /// Theme identity.
    pub key:           ThemeUuid,
    /// Display label.
    pub display_name:  ThemeDisplayName,
    /// Default floor terrain for empty cells.
    pub default_floor: TerrainUuid,
    /// Terrain pieces available in this theme.
    pub terrain:       Vec<TerrainUuid>,
}
