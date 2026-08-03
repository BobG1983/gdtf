//! Quiet material restamp when sprite defs hot-reload.

use bevy::prelude::*;
use gdtf_battle_sim::prelude::CellLevel;
use gdtf_content_families::sprites::SpriteName;

use super::{static_draw::TerrainSprite, static_map::SpriteResolveCtx};
use crate::{TerrainFogMaterial, cell_to_world};

/// Graphic name currently stamped on a terrain tile.
#[derive(Component, Debug, Clone, PartialEq, Eq, Deref)]
pub struct StampedGraphic(SpriteName);

impl StampedGraphic {
    /// Build from an authored graphic key.
    #[must_use]
    pub fn from_key(name: &str) -> Self {
        Self(SpriteName::new(name.to_owned()))
    }

    pub(super) fn as_key(&self) -> &str {
        self.0.as_str()
    }
}

pub(super) fn stamp_tile_quiet(
    resolve: &SpriteResolveCtx,
    materials: &mut ResMut<Assets<TerrainFogMaterial>>,
    name: &str,
    at: CellLevel,
    material: &MeshMaterial2d<TerrainFogMaterial>,
    transform: &mut Mut<'_, Transform>,
) {
    let (target, offset) = resolve.resolved(name, &at);
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
            stamped.as_key(),
            tile.at,
            material,
            &mut transform,
        );
    }
}
