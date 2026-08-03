//! domain values): private inner, derived [`Deref`], `#[serde(transparent)]`, and
use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// value). Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare
#[derive(Deref, Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TerrainName(String);

impl TerrainName {
        #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }

    /// Whether this name is the empty-string sentinel (the `#[serde(default)]` for
            #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// `#[serde(transparent)]` round-trips a bare RON string. `Serialize` is added
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TerrainGraphicKey(String);

impl TerrainGraphicKey {
        #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}

/// inner + derived [`Deref`]; `#[serde(transparent)]` round-trips a bare RON string.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct FootfallSound(String);

impl FootfallSound {
        #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}
