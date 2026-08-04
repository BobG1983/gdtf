//! Top-down sheet atlases loaded at startup.

use bevy::{
    image::{ImageLoaderSettings, ImageSampler, TextureAtlasLayout},
    platform::collections::HashMap,
    prelude::*,
};
use serde::{Deserialize, Serialize};

/// Index into a sheet grid.
///
/// `#[serde(transparent)]` so an authored role-table field parses as a bare integer.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TileIndex(usize);

impl TileIndex {
    /// Build from a zero-based tile index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// Which PNG sheet a tile comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SheetRole {
    /// Terrain tileset.
    Terrain,
    /// Character / ganger tileset.
    Characters,
    /// Effects tileset.
    Effects,
    /// Portrait tileset.
    Portraits,
}

impl SheetRole {
    const ALL: [Self; 4] = [
        Self::Terrain,
        Self::Characters,
        Self::Effects,
        Self::Portraits,
    ];

    /// Asset path for this sheet.
    #[must_use]
    pub const fn asset_path(self) -> &'static str {
        match self {
            Self::Terrain => "sprites/alt_tileset_terrain.png",
            Self::Characters => "sprites/alt_tileset_characters.png",
            Self::Effects => "sprites/alt_tileset_effects.png",
            Self::Portraits => "sprites/alt_tileset_portraits.png",
        }
    }

    /// Grid columns and rows for this sheet.
    #[must_use]
    pub const fn grid(self) -> (u32, u32) {
        match self {
            Self::Terrain => (16, 22),
            Self::Characters => (16, 18),
            Self::Effects => (16, 8),
            Self::Portraits => (10, 10),
        }
    }

    /// Pixel size of one tile on this sheet.
    #[must_use]
    pub const fn tile_px(self) -> u32 {
        match self {
            Self::Terrain | Self::Characters | Self::Effects => 16,
            Self::Portraits => 32,
        }
    }

    pub(super) fn sampler_override(self) -> Option<ImageSampler> {
        match self {
            Self::Portraits => Some(ImageSampler::nearest()),
            Self::Terrain | Self::Characters | Self::Effects => None,
        }
    }
}

/// Image handle plus atlas layout for one sheet.
#[derive(Debug, Clone)]
pub struct SheetAtlas {
    /// Loaded sheet image.
    pub image:  Handle<Image>,
    /// Grid layout for the sheet.
    pub layout: Handle<TextureAtlasLayout>,
}

/// All loaded top-down sheets.
#[derive(Resource, Debug, Clone)]
pub struct TopDownAtlases {
    pub(super) sheets: HashMap<SheetRole, SheetAtlas>,
}

impl TopDownAtlases {
    /// Atlas for a sheet role, if loaded.
    #[must_use]
    pub fn role(&self, role: SheetRole) -> Option<&SheetAtlas> {
        self.sheets.get(&role)
    }

    /// Which sheet role owns this image asset id.
    #[must_use]
    pub fn sheet_role_for_image(&self, id: AssetId<Image>) -> Option<SheetRole> {
        self.sheets
            .iter()
            .find_map(|(role, sheet)| (sheet.image.id() == id).then_some(*role))
    }
}

/// Startup system: load every sheet and insert [`TopDownAtlases`].
pub fn load_topdown_atlases(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let mut sheets = HashMap::default();
    for role in SheetRole::ALL {
        let (columns, rows) = role.grid();
        let layout =
            TextureAtlasLayout::from_grid(UVec2::splat(role.tile_px()), columns, rows, None, None);
        sheets.insert(
            role,
            SheetAtlas {
                image:  load_sheet_image(&asset_server, role),
                layout: layouts.add(layout),
            },
        );
    }

    commands.insert_resource(TopDownAtlases { sheets });
}

fn load_sheet_image(asset_server: &AssetServer, role: SheetRole) -> Handle<Image> {
    match role.sampler_override() {
        Some(sampler) => asset_server
            .load_builder()
            .with_settings(move |settings: &mut ImageLoaderSettings| {
                settings.sampler = sampler.clone();
            })
            .load(role.asset_path()),
        None => asset_server.load(role.asset_path()),
    }
}
