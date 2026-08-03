//! Authored name, graphic, and footfall labels for terrain pieces.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// Human-readable terrain piece name.
#[derive(Deref, Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TerrainName(String);

impl TerrainName {
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }

    /// Whether this is the empty default name.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Key into the terrain graphic atlas.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TerrainGraphicKey(String);

impl TerrainGraphicKey {
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}

/// Footfall sound key for this surface.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct FootfallSound(String);

impl FootfallSound {
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}
