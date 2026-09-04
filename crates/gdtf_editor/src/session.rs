//! Shared authoring session state for the map editor.

use bevy::prelude::*;
use gdtf_battle_sim::{
    level::{GridSize, ThemeUuid},
    metric::CellLevel,
    terrain::{def::TerrainUuid, facing::TerrainFacing},
};

use crate::placement::ProposedPlacement;

/// Theme, grid size, selected tile and paint facing for the prefab canvas.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct MapEditorSession {
    theme:         ThemeUuid,
    default_floor: Option<TerrainUuid>,
    grid_size:     GridSize,
    selected_tile: Option<TerrainUuid>,
    facing:        TerrainFacing,
}

impl MapEditorSession {
    /// Build a session with no selected tile, painting north.
    #[must_use]
    pub const fn new(
        theme: ThemeUuid,
        default_floor: Option<TerrainUuid>,
        grid_size: GridSize,
    ) -> Self {
        Self {
            theme,
            default_floor,
            grid_size,
            selected_tile: None,
            facing: TerrainFacing::North,
        }
    }

    /// Active theme key.
    #[must_use]
    pub const fn theme(&self) -> ThemeUuid {
        self.theme
    }

    /// Default floor tile for the theme, if set.
    #[must_use]
    pub const fn default_floor(&self) -> Option<TerrainUuid> {
        self.default_floor
    }

    /// Prefab grid size.
    #[must_use]
    pub const fn grid_size(&self) -> GridSize {
        self.grid_size
    }

    /// Switch theme and its default floor.
    pub const fn select_theme(&mut self, theme: ThemeUuid, default_floor: Option<TerrainUuid>) {
        self.theme = theme;
        self.default_floor = default_floor;
    }

    /// Set the prefab grid size.
    pub const fn set_grid_size(&mut self, grid_size: GridSize) {
        self.grid_size = grid_size;
    }

    /// Currently selected paint tile.
    #[must_use]
    pub const fn selected_tile(&self) -> Option<TerrainUuid> {
        self.selected_tile
    }

    /// Select a paint tile.
    pub const fn select_tile(&mut self, tile: TerrainUuid) {
        self.selected_tile = Some(tile);
    }

    /// Clear the selected paint tile.
    pub const fn clear_selected_tile(&mut self) {
        self.selected_tile = None;
    }

    /// The facing a paint turns its piece to.
    #[must_use]
    pub const fn facing(&self) -> TerrainFacing {
        self.facing
    }

    /// Choose the facing a paint turns its piece to.
    pub const fn set_facing(&mut self, facing: TerrainFacing) {
        self.facing = facing;
    }

    /// The placement a paint at `slot` lays: the selected tile, turned the chosen way.
    #[must_use]
    pub fn paint_proposal(&self, slot: CellLevel) -> Option<ProposedPlacement> {
        Some(ProposedPlacement::new(
            slot,
            self.selected_tile?,
            self.facing,
        ))
    }
}

impl Default for MapEditorSession {
    fn default() -> Self {
        Self {
            theme:         ThemeUuid::nil(),
            default_floor: None,
            grid_size:     GridSize::default(),
            selected_tile: None,
            facing:        TerrainFacing::default(),
        }
    }
}
