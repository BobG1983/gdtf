use bevy::{ecs::system::SystemParam, image::Image, platform::collections::HashMap, prelude::*};
use gdtf_battle_sim::{
    entity::TerrainCell,
    occupancy::TerrainKind,
    piece::{FootfallSound, TerrainGraphicKey},
    prelude::{CellLevel, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::resolve::{
    MissingTileTexture, anchor_world_offset, resolve_sprite, single_rect_layout, source_parts,
    source_px_size, source_urect,
};
use crate::{Brightness, CELL_PX, TerrainFogMaterial};

/// A `#[derive(SystemParam)]` borrow-bundle (the system-analogue of a cohesive ctor
#[derive(SystemParam)]
pub struct StaticMap<'w, 's> {
    occupancy: Res<'w, OccupancyGrid>,
    surface:   Res<'w, SurfaceGrid>,
    terrain: Query<
        'w,
        's,
        (
            &'static TerrainCell,
            &'static TerrainGraphicKey,
            Option<&'static FootfallSound>,
        ),
    >,
}

impl StaticMap<'_, '_> {
    pub(super) fn graphic_facts(
        &self,
    ) -> HashMap<CellLevel, (&TerrainGraphicKey, Option<&FootfallSound>)> {
        self.terrain
            .iter()
            .map(|(cell, graphic, footfall)| (**cell, (graphic, footfall)))
            .collect()
    }
}

pub(super) fn graphic_name_at<'a>(
    key: &CellLevel,
    facts: &HashMap<CellLevel, (&'a TerrainGraphicKey, Option<&FootfallSound>)>,
) -> Option<&'a str> {
    facts.get(key).map(|(graphic, _footfall)| graphic.as_str())
}

/// A `#[derive(SystemParam)]` bundle (the [`StaticMap`] shape) so the draw + the three
#[derive(SystemParam)]
pub struct SpriteResolveCtx<'w> {
    defs:         Res<'w, SpriteDefRegistry>,
    asset_server: Res<'w, AssetServer>,
    images:       Res<'w, Assets<Image>>,
    missing:      Res<'w, MissingTileTexture>,
}

impl SpriteResolveCtx<'_> {
    pub(super) fn defs_changed(&self) -> bool {
        self.defs.is_changed()
    }

    pub(super) fn resolved(&self, name: &str, at: &CellLevel) -> (TerrainFogMaterial, Vec2) {
        let (image, region, offset) = self.resolved_parts(name, at);
        let material = TerrainFogMaterial {
            image,
            atlas_layout: region.map(single_rect_layout),
            atlas_index: 0,
            custom_size: Some(Vec2::splat(CELL_PX)),
            saturation: 1.0,
            brightness: Brightness::FULL,
        };
        (material, offset)
    }

    /// The magenta missing-tile material a cell with no resolved floor draws.
    pub(super) fn marker_material(&self) -> (TerrainFogMaterial, Vec2) {
        let material = TerrainFogMaterial {
            image:        self.missing.handle(),
            atlas_layout: None,
            atlas_index:  0,
            custom_size:  Some(Vec2::splat(CELL_PX)),
            saturation:   1.0,
            brightness:   Brightness::FULL,
        };
        (material, Vec2::ZERO)
    }

    pub(super) fn resolved_sprite(&self, name: &str, at: &CellLevel) -> (Sprite, Vec2) {
        let (image, region, offset) = self.resolved_parts(name, at);
        let mut sprite = Sprite::from_image(image);
        sprite.rect = region.map(|region| region.as_rect());
        sprite.custom_size = Some(Vec2::splat(CELL_PX));
        (sprite, offset)
    }

    fn resolved_parts(&self, name: &str, at: &CellLevel) -> (Handle<Image>, Option<URect>, Vec2) {
        let Some(def) = resolve_sprite(&self.defs, name) else {
            warn!(
                "terrain draw: no sprite def named `{name}` at {at:?} — drawing the magenta \
                 missing-sprite marker instead",
            );
            return (self.missing.handle(), None, Vec2::ZERO);
        };
        let (path, rect) = source_parts(&def.source);
        let image = self.asset_server.load(path.as_str().to_owned());
        let px = source_px_size(&def.source).or_else(|| self.images.get(&image).map(Image::size));
        let offset = px.map_or(Vec2::ZERO, |px| {
            anchor_world_offset(def, px, Vec2::splat(CELL_PX))
        });
        (image, rect.map(source_urect), offset)
    }
}

pub(super) fn storey_has_terrain(
    key: &CellLevel,
    facts: &HashMap<CellLevel, (&TerrainGraphicKey, Option<&FootfallSound>)>,
    map: &StaticMap,
) -> bool {
    facts.contains_key(key)
        || matches!(map.surface.slab_state(key), SlabState::Present)
        || !matches!(map.occupancy.terrain(key), TerrainKind::Open)
}

pub(super) fn i32_extent(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}
