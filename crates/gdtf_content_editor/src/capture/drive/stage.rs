use bevy::prelude::*;
use gdtf_battle_sim::{
    level::{GridHeight, GridLevels, GridSize, GridWidth, UuidThemeRegistry},
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::super::forced::ForcedView;
use crate::{
    EditorMap, hovered_cell::HoveredCell, session::MapEditorSession,
    terrain_graphics::terrain_sprite_def,
};

pub(in crate::capture) const SHOT_GRID_EDGE: u8 = 16;
const BLOCK: i32 = 3;

const SHOT_STOREYS_FULL: u8 = 2;

const UPPER_STOREY: u8 = 1;
pub(in crate::capture) fn drive_capture_grid_size(
    forced_view: Option<Res<ForcedView>>,
    session: Option<ResMut<MapEditorSession>>,
) {
    let Some(mut session) = session else {
        return;
    };
    let levels = if forced_view.is_some() {
        SHOT_STOREYS_FULL
    } else {
        1
    };
    let current = session.grid_size();
    if *current.width() == SHOT_GRID_EDGE
        && *current.height() == SHOT_GRID_EDGE
        && *current.levels() == levels
    {
        return;
    }
    if let Ok(size) = GridSize::new(
        GridWidth::new(SHOT_GRID_EDGE),
        GridHeight::new(SHOT_GRID_EDGE),
        GridLevels::new(levels),
    ) {
        session.set_grid_size(size);
    }
}

pub(in crate::capture) fn drive_capture_selection(
    registry: Option<Res<TerrainDefRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    sprites: Option<Res<SpriteDefRegistry>>,
    session: Option<ResMut<MapEditorSession>>,
) {
    let (Some(registry), Some(themes), Some(sprites), Some(mut session)) =
        (registry, themes, sprites, session)
    else {
        return;
    };
    if session.selected_tile().is_some() {
        return;
    }
    if let Some(paint_key) = distinct_paint_tile(&registry, &themes, &sprites, &session) {
        session.select_tile(paint_key);
    }
}

pub(in crate::capture) fn drive_capture_paint_and_hover(
    forced_view: Option<Res<ForcedView>>,
    session: Option<Res<MapEditorSession>>,
    mut map: Option<ResMut<EditorMap>>,
    mut hovered: Option<ResMut<HoveredCell>>,
) {
    let (Some(session), Some(map), Some(hovered)) = (session, map.as_mut(), hovered.as_mut())
    else {
        return;
    };
    let Some(paint_key) = session.selected_tile() else {
        return;
    };
    let size = session.grid_size();

    if map.painted_count() == 0 {
        for cell in block_cells() {
            map.paint(cell, paint_key, size);
        }
        if forced_view.is_some() {
            let upper = Level::new(UPPER_STOREY);
            for cell in upper_block_cells() {
                map.paint_at(CellLevel::new(cell, upper), paint_key, size);
            }
        }
    }

    hovered.set(Cell::new(BLOCK, 1), Level::new(0));
}

fn block_cells() -> Vec<Cell> {
    (0..BLOCK)
        .flat_map(|x| (0..BLOCK).map(move |y| Cell::new(x, y)))
        .collect()
}

fn upper_block_cells() -> Vec<Cell> {
    (0..BLOCK)
        .flat_map(|x| (0..BLOCK).map(move |y| Cell::new(x + BLOCK + 1, y)))
        .collect()
}

fn distinct_paint_tile(
    registry: &TerrainDefRegistry,
    themes: &UuidThemeRegistry,
    sprites: &SpriteDefRegistry,
    session: &MapEditorSession,
) -> Option<TerrainUuid> {
    let theme = session.theme();
    let default_def = session
        .default_floor()
        .or_else(|| themes.default_floor(&theme))
        .and_then(|key| terrain_sprite_def(registry, sprites, &key))?;
    themes.terrain(&theme)?.iter().find_map(|key| {
        terrain_sprite_def(registry, sprites, key)
            .filter(|def| *def != default_def)
            .map(|_| *key)
    })
}
