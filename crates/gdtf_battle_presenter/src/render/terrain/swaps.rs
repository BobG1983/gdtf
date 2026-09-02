//! The played-destruction stamp and the tile retarget its siblings share.

use bevy::prelude::*;
use gdtf_battle_sim::{occupancy_sync::TerrainPieceDestroyed, prelude::CellLevel};

use super::{
    band::{DrawnStoreys, cell_level_in_band},
    restamp::{StampedGraphic, stamp_tile_quiet},
    static_draw::TerrainSprite,
    static_map::{SpriteResolveCtx, StaticMap, graphic_name_at},
};
use crate::{TerrainFogMaterial, playback::Played};

pub(super) fn retarget_tile(
    at: CellLevel,
    stamp: &StampedGraphic,
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
        stamp_tile_quiet(resolve, materials, stamp, at, mat_handle, &mut transform);
        stamped.set_if_neq(stamp.clone());
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
        let stamp = graphic_name_at(&event.at, &graphic_facts)
            .map_or(StampedGraphic::Marker, StampedGraphic::from_key);
        retarget_tile(event.at, &stamp, &resolve, &mut materials, &mut tiles);
    }
}
