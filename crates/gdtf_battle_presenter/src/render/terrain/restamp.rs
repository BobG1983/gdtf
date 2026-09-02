//! Quiet material restamp when sprite defs hot-reload.

use bevy::prelude::*;
use gdtf_battle_sim::prelude::CellLevel;
use gdtf_content_families::sprites::SpriteName;

use super::{static_draw::TerrainSprite, static_map::SpriteResolveCtx};
use crate::{TerrainFogMaterial, cell_to_world};

/// What a terrain tile is currently stamped with: a sprite name, or the missing-tile marker.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub enum StampedGraphic {
    /// The sprite def this tile resolves through.
    Named(SpriteName),
    /// The magenta missing-tile marker, drawn where no piece and no theme default resolved.
    Marker,
}

impl StampedGraphic {
    /// Build from an authored graphic key.
    #[must_use]
    pub fn from_key(name: &str) -> Self {
        Self::Named(SpriteName::new(name.to_owned()))
    }

    /// The sprite name this stamp holds, or nothing when it is the marker.
    #[must_use]
    pub fn named(&self) -> Option<&str> {
        match self {
            Self::Named(name) => Some(name.as_str()),
            Self::Marker => None,
        }
    }
}

pub(super) fn stamp_tile_quiet(
    resolve: &SpriteResolveCtx,
    materials: &mut ResMut<Assets<TerrainFogMaterial>>,
    stamp: &StampedGraphic,
    at: CellLevel,
    material: &MeshMaterial2d<TerrainFogMaterial>,
    transform: &mut Mut<'_, Transform>,
) {
    let (target, offset) = match stamp.named() {
        Some(name) => resolve.resolved(name, &at),
        None => resolve.marker_material(),
    };
    let stale = materials.get(material.id()).is_some_and(|current| {
        current.image != target.image
            || current.atlas_layout != target.atlas_layout
            || current.atlas_index != target.atlas_index
    });
    if stale && let Some(mut current) = materials.get_mut(material.id()) {
        current.image = target.image;
        current.atlas_layout = target.atlas_layout;
        current.atlas_index = target.atlas_index;
    }
    let translation = cell_to_world(at.cell(), at.level()) + offset.extend(0.0);
    if transform.translation != translation {
        transform.translation = translation;
    }
}

/// Restamp every terrain tile when sprite defs change.
pub fn restamp_tiles_on_def_change(
    resolve: SpriteResolveCtx,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut tiles: Query<(
        &TerrainSprite,
        &StampedGraphic,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Transform,
    )>,
) {
    if !resolve.defs_changed() {
        return;
    }
    for (tile, stamped, material, mut transform) in tiles.iter_mut() {
        stamp_tile_quiet(
            &resolve,
            &mut materials,
            stamped,
            tile.at,
            material,
            &mut transform,
        );
    }
}
