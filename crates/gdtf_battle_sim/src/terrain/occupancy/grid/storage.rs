//! Occupancy grid storage: slots, occupants, terrain, and blocking maps.

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::{Deref, Entity, Resource},
};

use super::types::{GRID_HEIGHT, GRID_WIDTH, OccupancySlot, SLOT_COUNT};
use crate::{
    cover::HeightBand,
    metric::{CellLevel, MAX_LEVELS},
    occupancy::{OccupancyInput, PathBlocking, TerrainKind, VisionBlocking},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SlotIndex(usize);

impl SlotIndex {
    #[must_use]
    pub(super) const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// Authoritative occupancy grid: terrain, occupants, stairs, path and vision blocking.
#[derive(Resource, Debug, Clone)]
pub struct OccupancyGrid {
    pub(super) slots:           Box<[OccupancySlot]>,
    pub(super) occupant_bands:  HashMap<CellLevel, HeightBand>,
    pub(super) stair_cells:     HashSet<CellLevel>,
    pub(super) path_blocking:   PathBlocking,
    pub(super) vision_blocking: VisionBlocking,
}

impl Default for OccupancyGrid {
    fn default() -> Self {
        Self {
            slots:           vec![OccupancySlot::default(); SLOT_COUNT].into_boxed_slice(),
            occupant_bands:  HashMap::default(),
            stair_cells:     HashSet::default(),
            path_blocking:   PathBlocking::new(),
            vision_blocking: VisionBlocking::new(),
        }
    }
}

impl OccupancyGrid {
    /// Empty grid.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a grid from placement input and known stair cells.
    #[must_use]
    pub fn build_from_occupancy_input(
        input: &OccupancyInput,
        stair_cells: &bevy::platform::collections::HashSet<CellLevel>,
    ) -> Self {
        let mut grid = Self::new();
        for &cell in stair_cells {
            grid.mark_stair_cell(cell);
        }
        for placement in &input.terrain {
            grid.set_terrain(placement.at, placement.terrain);
        }
        for placement in &input.occupants {
            if stair_cells.contains(&placement.at) && placement.band != HeightBand::Low {
                let _ =
                    grid.register_stair_presence(placement.at, placement.occupant, placement.band);
            } else {
                grid.set_occupant(placement.at, Some(placement.occupant));
                grid.set_occupant_band(placement.at, Some(placement.band));
            }
        }
        grid
    }

    fn slot_index(key: &CellLevel) -> Option<SlotIndex> {
        let x = usize::try_from(key.x).ok()?;
        let y = usize::try_from(key.y).ok()?;
        let level = usize::try_from(key.z).ok()?;
        if x >= GRID_WIDTH || y >= GRID_HEIGHT || level >= MAX_LEVELS as usize {
            return None;
        }
        Some(SlotIndex::new(
            x + y * GRID_WIDTH + level * GRID_WIDTH * GRID_HEIGHT,
        ))
    }

    /// Read a slot if the key is in bounds.
    #[must_use]
    pub fn slot(&self, key: &CellLevel) -> Option<&OccupancySlot> {
        Self::slot_index(key).and_then(|i| self.slots.get(*i))
    }

    /// Set terrain at a cell.
    pub fn set_terrain(&mut self, key: CellLevel, terrain: TerrainKind) {
        if let Some(slot) = Self::slot_index(&key).and_then(|i| self.slots.get_mut(*i)) {
            slot.terrain = terrain;
        }
    }

    /// Set or clear the occupant entity at a cell.
    pub fn set_occupant(&mut self, key: CellLevel, occupant: Option<Entity>) {
        if let Some(slot) = Self::slot_index(&key).and_then(|i| self.slots.get_mut(*i)) {
            slot.occupant = occupant;
        }
    }

    /// Terrain kind at a cell (Open if unset).
    #[must_use]
    pub fn terrain(&self, key: &CellLevel) -> TerrainKind {
        self.slot(key).map_or(TerrainKind::Open, |s| s.terrain)
    }

    /// Occupant entity at a cell, if any.
    #[must_use]
    pub fn occupant(&self, key: &CellLevel) -> Option<Entity> {
        self.slot(key).and_then(|s| s.occupant)
    }

    /// Silhouette height band of the occupant at a cell.
    #[must_use]
    pub fn occupant_band(&self, key: &CellLevel) -> Option<HeightBand> {
        self.occupant_bands.get(key).copied()
    }

    /// Set or clear the occupant height band.
    pub fn set_occupant_band(&mut self, key: CellLevel, band: Option<HeightBand>) {
        match band {
            Some(band) => {
                self.occupant_bands.insert(key, band);
            }
            None => {
                self.occupant_bands.remove(&key);
            }
        }
    }
}
