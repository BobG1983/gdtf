use bevy::prelude::Resource;

use super::{TerrainDef, TerrainUuid};
use crate::registry::Registry;

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct TerrainDefRegistry(Registry<TerrainUuid, TerrainDef>);

impl TerrainDefRegistry {
            #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (TerrainUuid, TerrainDef)>) -> Self {
        Self(Registry::new(defs))
    }

            pub fn insert(&mut self, key: TerrainUuid, def: TerrainDef) -> Option<TerrainDef> {
        self.0.insert(key, def)
    }

            #[must_use]
    pub fn def(&self, key: &TerrainUuid) -> Option<&TerrainDef> {
        self.0.get(key)
    }

                    pub fn defs(&self) -> impl Iterator<Item = (&TerrainUuid, &TerrainDef)> {
        self.0.iter()
    }

        #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

        #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
