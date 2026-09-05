//! Stamp each terrain tile with the view its piece's def, facing and open state select.

use bevy::{ecs::system::SystemParam, platform::collections::HashSet, prelude::*};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog},
    def::TerrainDefRegistry,
    prelude::CellLevel,
};

use super::{
    band::{DrawnStoreys, cell_level_in_band},
    restamp::StampedGraphic,
    static_draw::TerrainSprite,
    static_map::{LeftoverArt, SpriteResolveCtx},
    swaps::retarget_tile,
    view_resolve::{TerrainPieces, view_key_for},
};
use crate::{TerrainFogMaterial, playback::PlaybackCursor};

/// What stands in the cells a restamp reads: the terrain pieces with their change ticks,
/// the sprites destroyed pieces left behind, and the tiles respawned this tick.
#[derive(SystemParam)]
pub struct StandingTerrain<'w, 's> {
    pieces:    TerrainPieces<'w, 's>,
    leftovers: LeftoverArt<'w, 's>,
    respawned: Query<'w, 's, (), Added<TerrainSprite>>,
}

impl StandingTerrain<'_, '_> {
    // Whether any terrain tile was spawned this tick, which every cell must be redrawn for.
    fn any_respawned(&self) -> bool {
        !self.respawned.is_empty()
    }
}

/// The cells whose destruction the playback cursor has not reached, read from the act log
/// the view is playing rather than from raw sim state.
#[derive(SystemParam)]
pub struct UnplayedSmashes<'w> {
    log:    Option<Res<'w, ActLog>>,
    cursor: Option<Res<'w, PlaybackCursor>>,
}

impl UnplayedSmashes<'_> {
    // Cells the log records as smashed at or after the entry the cursor shows next.
    fn cells(&self) -> HashSet<CellLevel> {
        let (Some(log), Some(cursor)) = (self.log.as_ref(), self.cursor.as_ref()) else {
            return HashSet::default();
        };
        log.since(cursor.shown())
            .filter_map(|entry| match entry.deed() {
                ActDeed::TerrainPieceSmashed { at, .. } => Some(*at),
                _ => None,
            })
            .collect()
    }
}

/// Restamp the tile under each terrain piece with the view that piece's def names.
pub fn restamp_terrain_views(
    storeys: DrawnStoreys,
    defs: Res<TerrainDefRegistry>,
    resolve: SpriteResolveCtx,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    standing: StandingTerrain,
    unplayed: UnplayedSmashes,
    mut tiles: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
        &mut StampedGraphic,
    )>,
) {
    let band = storeys.band();
    let band_wide = *storeys.changed() || standing.any_respawned();
    let left_standing = standing.leftovers.by_cell();
    let waiting = unplayed.cells();
    for (cell, piece, facing, open) in &standing.pieces {
        let at = **cell;
        if !cell_level_in_band(at, &band)
            || left_standing.contains_key(&at)
            || waiting.contains(&at)
        {
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
