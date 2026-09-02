//! Live terrain graphic stamps for destruction and emplacement occupancy.

use bevy::prelude::*;
use gdtf_battle_sim::{
    emplacement::EmplacementState, entity::TerrainCell, occupancy_sync::TerrainPieceDestroyed,
    prelude::CellLevel,
};

use super::{
    active_level::{ActiveLevel, ViewMode},
    band::{DrawnStoreys, cell_level_in_band, drawn_band},
    restamp::{StampedGraphic, stamp_tile_quiet},
    roles::TileRole,
    static_draw::TerrainSprite,
    static_map::{SpriteResolveCtx, StaticMap, graphic_name_at},
    treatment::{IsolateView, StoreyViewMode},
};
use crate::{TerrainFogMaterial, playback::Played};

fn retarget_tile(
    at: CellLevel,
    name: &str,
    resolve: &SpriteResolveCtx,
    materials: &mut ResMut<Assets<TerrainFogMaterial>>,
    tiles: &mut Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
        &mut StampedGraphic,
    )>,
) {
    for (terrain, mat_handle, mut transform, mut stamped) in tiles.iter_mut() {
        if terrain.at != at {
            continue;
        }
        stamp_tile_quiet(resolve, materials, name, at, mat_handle, &mut transform);
        stamped.set_if_neq(StampedGraphic::from_key(name));
    }
}

/// Stamp a smashed cell with whatever stands there now, once the playback cursor
/// plays the [`Played`] `TerrainPieceDestroyed`.
pub fn stamp_destroyed_cell(
    storeys: DrawnStoreys,
    map: StaticMap,
    resolve: SpriteResolveCtx,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut destroyed: MessageReader<Played<TerrainPieceDestroyed>>,
    mut tiles: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
        &mut StampedGraphic,
    )>,
) {
    let band = storeys.band();
    let graphic_facts = map.graphic_facts();
    for event in destroyed.read() {
        if !cell_level_in_band(event.at, &band) {
            continue;
        }
        let name = graphic_name_at(&event.at, &graphic_facts, &map);
        retarget_tile(event.at, name, &resolve, &mut materials, &mut tiles);
    }
}

/// Swap emplacement tiles between empty and occupied graphics.
pub fn indicate_emplacement_occupied(
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    isolate: Res<IsolateView>,
    resolve: SpriteResolveCtx,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    emplacements: Query<(&EmplacementState, &TerrainCell), Changed<EmplacementState>>,
    mut tiles: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
        &mut StampedGraphic,
    )>,
) {
    let band = drawn_band(*active, StoreyViewMode::new(*view, *isolate));
    for (state, cell) in &emplacements {
        let at = **cell;
        if !cell_level_in_band(at, &band) {
            continue;
        }
        let role = if *state.is_occupied() {
            TileRole::EmplacementOccupied
        } else {
            TileRole::Emplacement
        };
        retarget_tile(at, role.as_key(), &resolve, &mut materials, &mut tiles);
    }
}
