//! Cell → entity lookup for cover and slabs.

use bevy::{
    platform::collections::HashMap,
    prelude::{Entity, Resource},
};

use crate::metric::CellLevel;

/// Key into the terrain index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainIndexKey {
    /// Cover at a cell.
    Cover(CellLevel),
    /// Slab at a cell.
    Slab(CellLevel),
}

impl TerrainIndexKey {
    /// Cell of this key.
    #[must_use]
    pub const fn cell_level(self) -> CellLevel {
        match self {
            Self::Cover(cl) | Self::Slab(cl) => cl,
        }
    }
}

/// Maps cover/slab cells to their entities.
#[derive(Resource, Debug, Clone, Default)]
pub struct TerrainIndex {
    entries: HashMap<TerrainIndexKey, Entity>,
}

impl TerrainIndex {
    /// From key/entity pairs.
    #[must_use]
    pub fn new(pairs: impl IntoIterator<Item = (TerrainIndexKey, Entity)>) -> Self {
        Self {
            entries: pairs.into_iter().collect(),
        }
    }

    /// Look up an entity.
    #[must_use]
    pub fn get(&self, key: &TerrainIndexKey) -> Option<Entity> {
        self.entries.get(key).copied()
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
