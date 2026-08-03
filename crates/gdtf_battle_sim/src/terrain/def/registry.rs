//! Registry of terrain definitions.

use bevy::prelude::Resource;

use super::{TerrainDef, TerrainUuid};
use crate::registry::Registry;

/// Map from terrain UUID to definition.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct TerrainDefRegistry(Registry<TerrainUuid, TerrainDef>);

impl TerrainDefRegistry {
    /// From key/def pairs.
    #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (TerrainUuid, TerrainDef)>) -> Self {
        Self(Registry::new(defs))
    }

    /// Insert or replace a def.
    pub fn insert(&mut self, key: TerrainUuid, def: TerrainDef) -> Option<TerrainDef> {
        self.0.insert(key, def)
    }

    /// Look up a def.
    #[must_use]
    pub fn def(&self, key: &TerrainUuid) -> Option<&TerrainDef> {
        self.0.get(key)
    }

    /// Iterate all defs.
    pub fn defs(&self) -> impl Iterator<Item = (&TerrainUuid, &TerrainDef)> {
        self.0.iter()
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
