//! Authored battlefield aggregate.

use bevy::reflect::TypePath;
use serde::Deserialize;

use super::{
    piece_spawns::{CoverSpawn, FieldSpawn, FloorSpawn, SlabSpawn},
    placed_ganger::PlacedGanger,
    roster_member::RosterMember,
};
use crate::{
    ganger::Faction,
    level::{GridSize, ThemeUuid},
    metric::CellLevel,
    terrain::def::TerrainUuid,
    vertical::VerticalLink,
};

/// Complete authored situation for one battle.
#[derive(Debug, Clone, Default, Deserialize, TypePath)]
#[serde(default)]
pub struct Situation {
    /// Explicitly placed gangers.
    pub gangers:        Vec<PlacedGanger>,
    /// Roster members without fixed cells (procgen places them).
    pub rosters:        Vec<RosterMember>,
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
    /// Player's faction index.
    pub player_faction: Faction,
    /// Default floor terrain when no per-cell floor is authored.
    pub default_floor:  TerrainUuid,
    /// Per-cell floor overrides.
    pub floors:         Vec<FloorSpawn>,
    /// Area-damage field placements.
    pub fields:         Vec<FieldSpawn>,
}

impl Situation {
    /// Empty situation with defaults.
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
