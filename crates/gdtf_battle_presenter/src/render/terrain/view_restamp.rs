//! Stamp each terrain tile with the view its piece's def, facing and open state select.

use bevy::prelude::*;
use gdtf_battle_sim::def::TerrainDefRegistry;

use super::{
    band::{DrawnStoreys, cell_level_in_band},
    restamp::StampedGraphic,
    static_draw::TerrainSprite,
    static_map::SpriteResolveCtx,
    swaps::retarget_tile,
    view_resolve::{TerrainPieces, view_key_for},
};
use crate::TerrainFogMaterial;

/// Restamp the tile under each terrain piece with the view that piece's def names.
pub fn restamp_terrain_views(
    storeys: DrawnStoreys,
    defs: Res<TerrainDefRegistry>,
    resolve: SpriteResolveCtx,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    pieces: TerrainPieces,
    mut tiles: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
        &mut StampedGraphic,
    )>,
    respawned: Query<(), Added<TerrainSprite>>,
) {
    let band = storeys.band();
    let band_wide = *storeys.changed() || !respawned.is_empty();
    for (cell, piece, facing, open) in &pieces {
        let at = **cell;
        if !cell_level_in_band(at, &band) {
            continue;
        }
        let row_changed =
            piece.is_changed() || facing.is_changed() || open.as_ref().is_some_and(Ref::is_changed);
        if !band_wide && !row_changed {
            continue;
        }
        let Some(def) = defs.def(&piece) else {
            continue;
        };
        let Some(key) = view_key_for(def, *facing, open.as_deref().copied(), None) else {
            continue;
        };
        let stamp = StampedGraphic::from_key(key.as_str());
        retarget_tile(at, &stamp, &resolve, &mut materials, &mut tiles);
    }
}
