//! Live terrain graphic swaps for destruction and emplacement occupancy.

use bevy::prelude::*;
use gdtf_battle_sim::{
    emplacement::EmplacementState,
    entity::TerrainCell,
    occupancy_sync::{CoverDestroyed, SlabDestroyed},
    prelude::CellLevel,
};

use super::{
    active_level::{ActiveLevel, ViewMode},
    band::{cell_level_in_band, drawn_band},
    restamp::{StampedGraphic, stamp_tile_quiet},
    roles::TileRole,
    static_draw::TerrainSprite,
    static_map::SpriteResolveCtx,
    treatment::{IsolateView, StoreyViewMode},
};
use crate::TerrainFogMaterial;

fn retarget_tile(
    at: CellLevel,
    role: TileRole,
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
        stamp_tile_quiet(
            resolve,
            materials,
            role.as_key(),
            at,
            mat_handle,
            &mut transform,
        );
        stamped.set_if_neq(StampedGraphic::from_key(role.as_key()));
    }
}

/// Swap cover tiles to rubble when [`CoverDestroyed`] fires.
pub fn swap_destroyed_cover(
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    isolate: Res<IsolateView>,
    resolve: SpriteResolveCtx,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut destroyed: MessageReader<CoverDestroyed>,
    mut tiles: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
        &mut StampedGraphic,
    )>,
) {
    let band = drawn_band(*active, StoreyViewMode::new(*view, *isolate));
    for event in destroyed.read() {
        if !cell_level_in_band(event.at, &band) {
            continue;
        }
        retarget_tile(
            event.at,
            TileRole::Rubble,
            &resolve,
            &mut materials,
            &mut tiles,
        );
    }
}

/// Swap slab tiles to the destroyed graphic when [`SlabDestroyed`] fires.
pub fn swap_destroyed_slab(
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    isolate: Res<IsolateView>,
    resolve: SpriteResolveCtx,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut destroyed: MessageReader<SlabDestroyed>,
    mut tiles: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
        &mut StampedGraphic,
    )>,
) {
    let band = drawn_band(*active, StoreyViewMode::new(*view, *isolate));
    for event in destroyed.read() {
        if !cell_level_in_band(event.at, &band) {
            continue;
        }
        retarget_tile(
            event.at,
            TileRole::SlabDestroyed,
            &resolve,
            &mut materials,
            &mut tiles,
        );
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
        retarget_tile(at, role, &resolve, &mut materials, &mut tiles);
    }
}
