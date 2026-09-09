//! Authored map half of a situation: terrain, theme, board size and fields.

use serde::{Deserialize, Serialize};

use super::piece_spawns::{CoverSpawn, FieldSpawn, FloorSpawn, SlabSpawn};
use crate::{
    level::{GridSize, ThemeUuid},
    metric::CellLevel,
    terrain::def::TerrainUuid,
    vertical::VerticalLink,
};

/// The battlefield one battle is fought on, with no combatant in it.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct BattleMap {
    /// Theme key.
    pub theme:          ThemeUuid,
    /// Board size.
    pub grid_size:      GridSize,
    /// Wall cover placements.
    pub walls:          Vec<CoverSpawn>,
    /// Scatter cover placements.
    pub scatter:        Vec<CoverSpawn>,
    /// Slab placements.
    pub slabs:          Vec<SlabSpawn>,
    /// Vertical links between levels.
    pub vertical_links: Vec<VerticalLink>,
    /// Default floor terrain when no per-cell floor is authored.
    pub default_floor:  TerrainUuid,
    /// Per-cell floor overrides.
    pub floors:         Vec<FloorSpawn>,
    /// Area-damage field placements.
    pub fields:         Vec<FieldSpawn>,
}

impl BattleMap {
    /// Empty map with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cells that have authored cover or slabs.
    pub fn authored_cells(&self) -> impl Iterator<Item = CellLevel> + '_ {
        self.walls
            .iter()
            .map(|c| c.at)
            .chain(self.scatter.iter().map(|c| c.at))
            .chain(self.slabs.iter().map(|s| s.at))
    }
}
