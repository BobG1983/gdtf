//! The editor's **terrain tile atlas** — the loaded sprite sheet + grid layout the palette
//! rows draw their tile sprites from (GTW-422 C1).
//!
//! The map editor lists every terrain of the active theme's palette, each row showing the
//! terrain's real SPRITE beside its name (C1). A [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef)
//! carries NO per-def atlas index (the sim is render-free); its
//! [`presenter_kind`](gdtf_battle_sim::terrain::def::TerrainDef::presenter_kind) names a GRAPHIC
//! ROLE KEY the PRESENTER resolves to an atlas index through its
//! [`TileRoles`](gdtf_battle_presenter::TileRoles) table. The editor mirrors that resolution (see
//! [`terrain_graphics`](crate::terrain_graphics)) and draws the resolved index over this sheet.
//!
//! This loads the SAME terrain sheet the presenter draws
//! — `sprites/alt_tileset_terrain.png`, a 16×22 grid of 16-px cells (the
//! `gdtf_battle_presenter` `SheetRole::Terrain` shape) — via [`AssetServer::load`] +
//! [`TextureAtlasLayout::from_grid`], the `load_topdown_atlases` recipe. A palette / canvas cell
//! then builds an [`ImageNode::from_atlas_image`] over this sheet at the resolved atlas index (a
//! UI atlas image node, NOT a world sprite).

use bevy::{image::TextureAtlasLayout, prelude::*};

/// Loose-file path of the terrain sprite sheet, relative to the asset source root.
///
/// The SAME sheet the presenter's `SheetRole::Terrain` loads
/// (`gdtf_battle_presenter::SheetRole::asset_path`), so the editor palette draws the tiles
/// from the identical art the battlescape renders.
const TERRAIN_SHEET_PATH: &str = "sprites/alt_tileset_terrain.png";

/// The terrain sheet's grid COLUMN count — the `gdtf_battle_presenter` `SheetRole::Terrain`
/// grid is 16×22 (352 cells). A framework layout dimension fed straight to
/// [`TextureAtlasLayout::from_grid`] (the `no-bare-types` clause-4 plumbing carve-out, the
/// presenter's `grid()` precedent), not a domain quantity.
const TERRAIN_COLUMNS: u32 = 16;

/// The terrain sheet's grid ROW count (16×22 — see [`TERRAIN_COLUMNS`]).
const TERRAIN_ROWS: u32 = 22;

/// One terrain atlas cell's square edge, in source pixels — the presenter's
/// `SheetRole::Terrain` `tile_px` (16). A framework layout dimension (the clause-4 plumbing
/// carve-out), not a domain quantity.
const TERRAIN_TILE_PX: u32 = 16;

/// The editor's loaded **terrain tile atlas** — the sheet image + the grid layout over it the
/// palette rows draw their sprites from (GTW-422 C1).
///
/// A named [`Resource`] (a framework type, exempt from `no-bare-types`; the handles it holds
/// are framework plumbing). Loaded ONCE on `OnEnter(Editing)` by [`load_tile_atlas`] and read
/// by the palette / canvas spawn to build each cell's [`ImageNode::from_atlas_image`] at the
/// graphic index resolved THE WAY THE PRESENTER DOES (see
/// [`terrain_graphics`](crate::terrain_graphics)). Mirrors the presenter's `SheetAtlas`
/// (image + layout pair) but editor-local and terrain-only.
#[derive(Resource, Debug, Clone)]
pub(crate) struct TileAtlas {
    /// The terrain sheet image handle (loaded via [`AssetServer::load`]).
    image:  Handle<Image>,
    /// The 16×22 grid layout over [`image`](TileAtlas::image), one entry per 16-px cell.
    layout: Handle<TextureAtlasLayout>,
}

impl TileAtlas {
    /// The terrain sheet image handle.
    pub(crate) fn image(&self) -> Handle<Image> {
        self.image.clone()
    }

    /// The terrain sheet's grid-layout handle.
    pub(crate) fn layout(&self) -> Handle<TextureAtlasLayout> {
        self.layout.clone()
    }
}

/// `OnEnter(Editing)`: load the terrain sheet + register its grid layout, inserting the
/// [`TileAtlas`] resource the palette rows draw from (GTW-422 C1).
///
/// Mirrors the presenter's `load_topdown_atlases` recipe for the terrain sheet only:
/// [`AssetServer::load`] the PNG (the RON loader is `.ron`-only) + one
/// [`TextureAtlasLayout::from_grid`] at the sheet's `(columns, rows)` and tile size, added to
/// [`Assets<TextureAtlasLayout>`]. Param-only (`bevy-traps.md` #7): [`Commands`] for the
/// resource insert, [`Res<AssetServer>`] for the image load, [`ResMut<Assets<…>>`] to register
/// the layout. The async image decode finishes later — the row's [`ImageNode`] holds the
/// handle and renders once the texture is ready, so this never blocks `Editing`.
pub(crate) fn load_tile_atlas(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let layout = TextureAtlasLayout::from_grid(
        UVec2::splat(TERRAIN_TILE_PX),
        TERRAIN_COLUMNS,
        TERRAIN_ROWS,
        None,
        None,
    );
    commands.insert_resource(TileAtlas {
        image:  asset_server.load(TERRAIN_SHEET_PATH),
        layout: layouts.add(layout),
    });
}
