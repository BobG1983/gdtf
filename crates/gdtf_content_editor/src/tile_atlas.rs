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
//! This loads the SAME terrain sheet the presenter draws — reading the sheet's path, grid,
//! and per-cell size STRAIGHT off the presenter's [`SheetRole::Terrain`] spec (GTW-566 C7;
//! the editor previously mirrored those dimensions as its own consts, which could drift) —
//! via [`AssetServer::load`] + [`TextureAtlasLayout::from_grid`], the `load_topdown_atlases`
//! recipe. A palette / canvas cell then builds an [`ImageNode::from_atlas_image`] over this
//! sheet at the resolved atlas index (a UI atlas image node, NOT a world sprite).

use bevy::{image::TextureAtlasLayout, prelude::*};
use gdtf_battle_presenter::SheetRole;

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
    /// The grid layout over [`image`](TileAtlas::image), one entry per
    /// [`SheetRole::tile_px`]-sized cell of the [`SheetRole::Terrain`] grid.
    layout: Handle<TextureAtlasLayout>,
}

impl TileAtlas {
    /// The terrain sheet image handle — the preview tile sprites (GTW-515) draw over it.
    pub(crate) fn image(&self) -> Handle<Image> {
        self.image.clone()
    }

    /// The terrain sheet's grid-layout handle — the preview tile sprites (GTW-515) index into it.
    pub(crate) fn layout(&self) -> Handle<TextureAtlasLayout> {
        self.layout.clone()
    }
}

/// `OnEnter(Editing)`: load the terrain sheet + register its grid layout, register the sheet image
/// with egui (for the palette rows' sprite thumbnails — C4.2), and insert the [`TileAtlas`]
/// resource (GTW-422 C1; egui-registered in GTW-515).
///
/// Mirrors the presenter's `load_topdown_atlases` recipe for the terrain sheet only, reading
/// the sheet spec — [`SheetRole::asset_path`] / [`SheetRole::grid`] / [`SheetRole::tile_px`] —
/// straight off [`SheetRole::Terrain`] (GTW-566 C7, no duplicated dimensions):
/// [`AssetServer::load`] the PNG (the RON loader is `.ron`-only) + one
/// [`TextureAtlasLayout::from_grid`] at the sheet's `(columns, rows)` and tile size, added to
/// [`Assets<TextureAtlasLayout>`]. It ALSO registers the sheet image with egui via
/// [`EguiUserTextures`](bevy_egui::EguiUserTextures) (a STRONG handle — the [`TileAtlas`] keeps it
/// alive) so the egui palette can draw each terrain's sprite as an [`egui::Image`] UV sub-rect.
/// Param-only (`bevy-traps.md` #7). The async image decode finishes later — the palette / preview
/// hold the handle and render once the texture is ready, so this never blocks `Editing`.
pub(crate) fn load_tile_atlas(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    egui_user_textures: Option<ResMut<bevy_egui::EguiUserTextures>>,
) {
    // GTW-566 C7: the ONE sheet spec — the presenter's SheetRole::Terrain — supplies the
    // grid, cell size, and path; the editor holds no mirror consts to drift.
    let (columns, rows) = SheetRole::Terrain.grid();
    let layout = TextureAtlasLayout::from_grid(
        UVec2::splat(SheetRole::Terrain.tile_px()),
        columns,
        rows,
        None,
        None,
    );
    let image = asset_server.load(SheetRole::Terrain.asset_path());
    // Register the sheet with egui for the palette thumbnails. Absent under the headless harness
    // (no EguiPlugin) — the atlas still loads; only the egui registration is skipped there.
    if let Some(mut egui_user_textures) = egui_user_textures {
        egui_user_textures.add_image(bevy_egui::EguiTextureHandle::Strong(image.clone()));
    }
    commands.insert_resource(TileAtlas {
        image,
        layout: layouts.add(layout),
    });
}
