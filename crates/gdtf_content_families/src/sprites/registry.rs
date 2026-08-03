//! Runtime registry of sprite definitions.

use bevy::prelude::{Deref, Resource};
use gdtf_battle_sim::registry::Registry;
use serde::{Deserialize, Serialize};

use super::def::SpriteDef;

/// Stable name for a sprite definition.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SpriteName(String);

impl SpriteName {
    /// Wrap a name string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// Map of sprite name to definition.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct SpriteDefRegistry(Registry<SpriteName, SpriteDef>);

impl SpriteDefRegistry {
    /// Build a registry from name/def pairs.
    #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (SpriteName, SpriteDef)>) -> Self {
        Self(Registry::new(defs))
    }

    /// Insert or replace a definition. Returns the previous value if any.
    pub fn insert(&mut self, name: SpriteName, def: SpriteDef) -> Option<SpriteDef> {
        self.0.insert(name, def)
    }

    /// Look up a definition by name.
    #[must_use]
    pub fn def(&self, name: &SpriteName) -> Option<&SpriteDef> {
        self.0.get(name)
    }

    /// Whether a name is present.
    #[must_use]
    pub fn contains(&self, name: &SpriteName) -> bool {
        self.0.contains(name)
    }

    /// Iterate all definitions.
    pub fn defs(&self) -> impl Iterator<Item = (&SpriteName, &SpriteDef)> {
        self.0.iter()
    }

    /// Iterate all names.
    pub fn keys(&self) -> impl Iterator<Item = &SpriteName> {
        self.0.keys()
    }

    /// Number of definitions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
