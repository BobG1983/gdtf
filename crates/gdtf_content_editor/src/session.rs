//! The editor's shared **authoring session** — the canonical selection state the right
use bevy::prelude::*;
use gdtf_battle_sim::{
    level::{GridSize, ThemeUuid},
    terrain::def::TerrainUuid,
};

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct MapEditorSession {
        theme:         ThemeUuid,
            default_floor: Option<TerrainUuid>,
        grid_size:     GridSize,
                selected_tile: Option<TerrainUuid>,
}

impl MapEditorSession {
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
        }
    }

        #[must_use]
    pub const fn theme(&self) -> ThemeUuid {
        self.theme
    }

            #[must_use]
    pub const fn default_floor(&self) -> Option<TerrainUuid> {
        self.default_floor
    }

        #[must_use]
    pub const fn grid_size(&self) -> GridSize {
        self.grid_size
    }

            pub const fn select_theme(&mut self, theme: ThemeUuid, default_floor: Option<TerrainUuid>) {
        self.theme = theme;
        self.default_floor = default_floor;
    }

            pub const fn set_grid_size(&mut self, grid_size: GridSize) {
        self.grid_size = grid_size;
    }

            #[must_use]
    pub const fn selected_tile(&self) -> Option<TerrainUuid> {
        self.selected_tile
    }

            pub const fn select_tile(&mut self, tile: TerrainUuid) {
        self.selected_tile = Some(tile);
    }

                pub const fn clear_selected_tile(&mut self) {
        self.selected_tile = None;
    }
}

impl Default for MapEditorSession {
                fn default() -> Self {
        Self {
            theme:         ThemeUuid::nil(),
            default_floor: None,
            grid_size:     GridSize::default(),
            selected_tile: None,
        }
    }
}
