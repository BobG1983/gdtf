//! Death occurrence messages.

use bevy::prelude::{Entity, Message};

use crate::{metric::CellLevel, terrain::entity::TerrainIndexKey};

/// Something died at a cell (ganger, cover, or slab).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OnDeathOccurred {
    /// Entity that died (placeholder for terrain).
    pub entity:  Entity,
    /// Cell where death happened.
    pub at:      CellLevel,
    /// Terrain piece that died, when the death is a terrain death.
    pub terrain: Option<TerrainIndexKey>,
}

impl OnDeathOccurred {
    /// Death of a specific entity.
    #[must_use]
    pub const fn new(entity: Entity, at: CellLevel) -> Self {
        Self {
            entity,
            at,
            terrain: None,
        }
    }

    /// Cover destruction (no real entity).
    #[must_use]
    pub const fn cover(at: CellLevel) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            at,
            terrain: Some(TerrainIndexKey::Cover(at)),
        }
    }

    /// Slab destruction (no real entity).
    #[must_use]
    pub const fn slab(at: CellLevel) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            at,
            terrain: Some(TerrainIndexKey::Slab(at)),
        }
    }
}
