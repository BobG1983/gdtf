use bevy::prelude::Deref;
use serde::Deserialize;

use crate::occupancy::TerrainKind;

/// looked-up terrain cost), never a pinned magnitude. `#[serde(transparent)]` lets it
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct MoveCost(u8);

impl MoveCost {
                                #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

/// NOTE on which terrains can be a destination: a standing [`TerrainKind::Wall`] /
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct MoveCosts {
            pub open:  MoveCost,
            pub cover: MoveCost,
                pub wall:  MoveCost,
}

impl MoveCosts {
                                                                    #[must_use]
    pub const fn cost(&self, terrain: TerrainKind) -> MoveCost {
        match terrain {
            TerrainKind::Open => self.open,
            TerrainKind::Cover => self.cover,
            TerrainKind::Wall | TerrainKind::Emplacement => self.wall,
        }
    }
}

impl Default for MoveCosts {
    fn default() -> Self {
        Self {
            open:  MoveCost(4),
            cover: MoveCost(6),
            wall:  MoveCost(8),
        }
    }
}

/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct LinkTu(u8);

impl LinkTu {
                            #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

impl Default for LinkTu {
    fn default() -> Self {
        Self(4)
    }
}
