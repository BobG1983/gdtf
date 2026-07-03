//! The **shared egui sprite-thumbnail draw** (GTW-516 C3) — the ONE code path that turns a
//! terrain-sheet [`TileIndex`] into a fixed-size [`egui::Image`] over its atlas UV sub-rect.
//!
//! The GTW-515 PREFAB palette established the pattern (register the terrain sheet with egui once —
//! [`load_tile_atlas`](crate::tile_atlas::load_tile_atlas) — then draw a tile as an
//! [`egui::Image`] UV sub-rect over that sheet's [`egui::TextureId`]). GTW-516 reuses the SAME
//! path for the TERRAIN graphic-role picker's thumbnails, so the two share this helper rather than
//! carrying two sprite-loading paths (C3): the prefab palette resolves a tile from a
//! [`TerrainUuid`](gdtf_battle_sim::terrain::def::TerrainUuid) (via
//! [`terrain_atlas_index`](crate::terrain_graphics::terrain_atlas_index)); the terrain picker
//! resolves a role directly (via [`TileRoles::index_for_key`](gdtf_battle_presenter::TileRoles::index_for_key)).
//! Both end at a [`TileIndex`], which this module turns into the pixels — either drawn as a plain
//! [`egui::Image`] ([`draw_thumb`], the palette-row thumbnail) or built as an [`egui::Image`] for a
//! caller-owned widget ([`thumb_image`], the picker's clickable/selectable sprite button).

use bevy_egui::egui;
use gdtf_battle_presenter::TileIndex;

/// The pixel edge of a sprite thumbnail. A framework layout const (the egui image is sized in
/// screen points, not a domain quantity), shared by the prefab palette row + the terrain picker
/// grid so both draw the tile at the same size.
pub(crate) const THUMB_EDGE: f32 = 24.0;

/// Build the fixed-size ([`THUMB_EDGE`]) [`egui::Image`] for a terrain-sheet tile — the sheet's
/// [`egui::TextureId`] sampled over the tile's atlas UV sub-rect (GTW-516 C3). Returns [`None`]
/// when the resolved index / the egui sheet texture id is unavailable (the atlas has not yet
/// registered, or the role/def resolves to no index), so the caller draws a fallback rather than
/// panicking. `sheet_id` is the terrain sheet's egui texture id (registered once by
/// [`load_tile_atlas`](crate::tile_atlas::load_tile_atlas), resolved by the shell before the draw).
pub(crate) fn thumb_image(
    index: Option<TileIndex>,
    sheet_id: Option<egui::TextureId>,
) -> Option<egui::Image<'static>> {
    let (index, id) = index.zip(sheet_id)?;
    let uv = sheet_uv(*index);
    Some(
        egui::Image::new(egui::load::SizedTexture::new(
            id,
            egui::vec2(THUMB_EDGE, THUMB_EDGE),
        ))
        .uv(uv),
    )
}

/// Draw a terrain-sheet tile as a plain fixed-size ([`THUMB_EDGE`]) [`egui::Image`] (GTW-515 C4.2 /
/// GTW-516 C3) — the prefab palette row's thumbnail. When [`thumb_image`] cannot build the sprite
/// (index / sheet id unavailable) it allocates a blank fixed-size spacer instead, so the caller's
/// row layout stays stable (never a panic).
pub(crate) fn draw_thumb(
    ui: &mut egui::Ui,
    index: Option<TileIndex>,
    sheet_id: Option<egui::TextureId>,
) {
    match thumb_image(index, sheet_id) {
        Some(image) => {
            ui.add(image);
        }
        None => {
            ui.allocate_space(egui::vec2(THUMB_EDGE, THUMB_EDGE));
        }
    }
}

/// The UV sub-rect of a terrain-sheet atlas `index` — the sheet grid is read straight off the
/// presenter's [`SheetRole::Terrain`](gdtf_battle_presenter::SheetRole) spec
/// ([`grid`](gdtf_battle_presenter::SheetRole::grid), GTW-566 C7 — no editor mirror consts), so
/// cell `index` sits at column `index % cols`, row `index / cols` and spans one cell in
/// UV space. Shared by every sprite-thumbnail draw (GTW-516 C3).
fn sheet_uv(index: usize) -> egui::Rect {
    let (columns, rows) = gdtf_battle_presenter::SheetRole::Terrain.grid();
    let cols = columns as usize;
    let rows = rows as usize;
    let col = index % cols;
    let row = index / cols;
    #[expect(
        clippy::cast_precision_loss,
        reason = "sheet grid indices are small (16x22); the f32 cast is exact within f32's 24-bit \
                  integer range"
    )]
    let (u0, v0, uw, vh) = (
        col as f32 / cols as f32,
        row as f32 / rows as f32,
        1.0 / cols as f32,
        1.0 / rows as f32,
    );
    egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u0 + uw, v0 + vh))
}
