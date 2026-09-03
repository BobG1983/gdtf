//! Bodies lying on the grid: a corpse occludes a cell without occupying it.

use bevy::prelude::Entity;

use super::storage::OccupancyGrid;
use crate::{cover::HeightBand, metric::CellLevel};

/// A body on the floor of a cell, and the height band it occludes at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyOcclusion {
    body: Entity,
    band: HeightBand,
}

impl BodyOcclusion {
    /// Record `body` lying at `band`.
    #[must_use]
    pub const fn new(body: Entity, band: HeightBand) -> Self {
        Self { body, band }
    }

    /// The body lying here.
    #[must_use]
    pub const fn body(&self) -> Entity {
        self.body
    }

    /// The band the body occludes at.
    #[must_use]
    pub const fn band(&self) -> HeightBand {
        self.band
    }
}

impl OccupancyGrid {
    /// Body lying at a cell, if any.
    #[must_use]
    pub fn body(&self, key: &CellLevel) -> Option<BodyOcclusion> {
        self.bodies.get(key).copied()
    }

    /// Set or clear the body lying at a cell. One body per cell: a second replaces the first.
    pub fn set_body(&mut self, key: CellLevel, body: Option<BodyOcclusion>) {
        match body {
            Some(body) => {
                self.bodies.insert(key, body);
            }
            None => {
                self.bodies.remove(&key);
            }
        }
    }
}
