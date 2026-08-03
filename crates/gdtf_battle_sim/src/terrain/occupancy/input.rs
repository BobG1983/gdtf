use bevy::prelude::Entity;

use crate::{cover::HeightBand, metric::CellLevel, occupancy::TerrainKind};

/// [`TerrainKind`] authored there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainPlacement {
        pub at:      CellLevel,
        pub terrain: TerrainKind,
}

impl TerrainPlacement {
        #[must_use]
    pub const fn new(at: CellLevel, terrain: TerrainKind) -> Self {
        Self { at, terrain }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OccupantPlacement {
        pub at:       CellLevel,
            pub occupant: Entity,
            pub band:     HeightBand,
}

impl OccupantPlacement {
            #[must_use]
    pub const fn new(at: CellLevel, occupant: Entity, band: HeightBand) -> Self {
        Self { at, occupant, band }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OccupancyInput {
        pub terrain:   Vec<TerrainPlacement>,
        pub occupants: Vec<OccupantPlacement>,
}

impl OccupancyInput {
        #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
