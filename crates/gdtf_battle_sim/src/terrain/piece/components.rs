//! Authored name, graphic, and footfall labels for terrain pieces.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// Human-readable terrain piece name.
#[derive(Deref, Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TerrainName(String);

impl TerrainName {
    /// Wrap a name string.
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

/// Key into the terrain graphic atlas, named by an authored view row.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TerrainGraphicKey(String);

impl TerrainGraphicKey {
    /// Wrap a graphic key.
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}

/// The sprite a destroyed piece leaves standing in its cell, named by its def's
/// `leaves_behind`. It carries no mechanics, only art.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LeftoverSprite(TerrainGraphicKey);

impl LeftoverSprite {
    /// Wrap the sprite key a destroyed piece leaves behind.
    #[must_use]
    pub const fn new(sprite: TerrainGraphicKey) -> Self {
        Self(sprite)
    }
}

/// Footfall sound key for this surface.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct FootfallSound(String);

impl FootfallSound {
    /// Wrap a sound key.
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}
