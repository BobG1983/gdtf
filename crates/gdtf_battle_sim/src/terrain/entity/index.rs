use bevy::{
    platform::collections::HashMap,
    prelude::{Entity, Resource},
};

use crate::metric::CellLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainIndexKey {
        Cover(CellLevel),
        Slab(CellLevel),
}

impl TerrainIndexKey {
        #[must_use]
    pub const fn cell_level(self) -> CellLevel {
        match self {
            Self::Cover(cl) | Self::Slab(cl) => cl,
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct TerrainIndex {
        entries: HashMap<TerrainIndexKey, Entity>,
}

impl TerrainIndex {
                    #[must_use]
    pub fn new(pairs: impl IntoIterator<Item = (TerrainIndexKey, Entity)>) -> Self {
        Self {
            entries: pairs.into_iter().collect(),
        }
    }

                        #[must_use]
    pub fn get(&self, key: &TerrainIndexKey) -> Option<Entity> {
        self.entries.get(key).copied()
    }

            #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

        #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
