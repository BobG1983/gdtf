use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    image::Image,
    platform::collections::HashMap,
    prelude::*,
};
use gdtf_battle_sim::{
    def::{TerrainDefRegistry, TerrainUuid},
    entity::TerrainCell,
    occupancy::TerrainKind,
    openable::OpenState,
    piece::{FootfallSound, LeftoverSprite},
    prelude::{CellLevel, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
    terrain::facing::TerrainFacing,
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::{
    resolve::{
        MissingTileTexture, anchor_world_offset, resolve_sprite, single_rect_layout, source_parts,
        source_px_size, source_urect,
    },
    view_resolve::view_key_for,
};
use crate::{Brightness, CELL_PX, TerrainFogMaterial};

/// What the piece standing at a cell brings to a sprite lookup: its def key, the way it
/// is turned, whether it stands open, and the surface it sounds like underfoot.
pub(super) struct PieceFacts<'a> {
    pub(super) piece:    TerrainUuid,
    pub(super) facing:   TerrainFacing,
    pub(super) open:     Option<OpenState>,
    pub(super) footfall: Option<&'a FootfallSound>,
}

/// The columns a terrain piece carries that the static draw reads: where it stands, which
/// def it is, how it is turned, whether it stands open, and its footfall.
#[derive(QueryData)]
pub struct TerrainPieceRow {
    cell:     &'static TerrainCell,
    piece:    &'static TerrainUuid,
    facing:   &'static TerrainFacing,
    open:     Option<&'static OpenState>,
    footfall: Option<&'static FootfallSound>,
}

/// The sprite keys destroyed pieces left standing, keyed by the cell each stands in.
pub(super) type LeftoverSprites<'a> = HashMap<CellLevel, &'a str>;

/// The sprites destroyed pieces left standing in their cells, carrying no mechanics.
#[derive(SystemParam)]
pub struct LeftoverArt<'w, 's> {
    left: Query<'w, 's, (&'static TerrainCell, &'static LeftoverSprite)>,
}

impl LeftoverArt<'_, '_> {
    /// Every cell holding a leftover sprite, and the sprite key it names.
    pub(super) fn by_cell(&self) -> LeftoverSprites<'_> {
        self.left
            .iter()
            .map(|(cell, sprite)| (**cell, sprite.as_str()))
            .collect()
    }
}

/// A `#[derive(SystemParam)]` borrow-bundle (the system-analogue of a cohesive ctor
#[derive(SystemParam)]
pub struct StaticMap<'w, 's> {
    occupancy: Res<'w, OccupancyGrid>,
    surface:   Res<'w, SurfaceGrid>,
    defs:      Res<'w, TerrainDefRegistry>,
    terrain:   Query<'w, 's, TerrainPieceRow>,
    leftovers: LeftoverArt<'w, 's>,
}

impl StaticMap<'_, '_> {
    /// The sprite each destroyed piece left standing, keyed by cell.
    pub(super) fn leftover_sprites(&self) -> LeftoverSprites<'_> {
        self.leftovers.by_cell()
    }

    pub(super) fn piece_facts(&self) -> HashMap<CellLevel, PieceFacts<'_>> {
        self.terrain
            .iter()
            .map(|row| {
                (
                    **row.cell,
                    PieceFacts {
                        piece:    *row.piece,
                        facing:   *row.facing,
                        open:     row.open.copied(),
                        footfall: row.footfall,
                    },
                )
            })
            .collect()
    }

    /// The def registry each cell's view is resolved against.
    pub(super) fn defs(&self) -> &TerrainDefRegistry {
        &self.defs
    }
}

/// The sprite key the cell at `key` draws: the sprite a destroyed piece left standing
/// there, or the view the piece standing there resolves through its own def.
pub(super) fn sprite_name_at<'a>(
    key: &CellLevel,
    facts: &HashMap<CellLevel, PieceFacts<'_>>,
    leftovers: &LeftoverSprites<'a>,
    defs: &'a TerrainDefRegistry,
) -> Option<&'a str> {
    if let Some(sprite) = leftovers.get(key) {
        return Some(sprite);
    }
    let at = facts.get(key)?;
    let def = defs.def(&at.piece)?;
    view_key_for(def, at.facing, at.open, None).map(|sprite| sprite.as_str())
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
    facts: &HashMap<CellLevel, PieceFacts<'_>>,
    leftovers: &LeftoverSprites<'_>,
    map: &StaticMap,
) -> bool {
    facts.contains_key(key)
        || leftovers.contains_key(key)
        || matches!(map.surface.slab_state(key), SlabState::Present)
        || !matches!(map.occupancy.terrain(key), TerrainKind::Open)
}

pub(super) fn i32_extent(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}
