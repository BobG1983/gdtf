//! The PREFAB-mode **palette panel** (GTW-515 C4.2) — the LEFT panel: one selectable row per
//! terrain in the selected theme's palette, each showing its sprite thumbnail (via
//! [`terrain_atlas_index`]) with the active row highlighted, plus a stat summary for the selected
//! tile.
//!
//! Reuses the theme / terrain registries + the presenter-mirroring [`terrain_atlas_index`]
//! resolution VERBATIM (C4.2): a row's sprite is the terrain's own graphic (resolved THE WAY THE
//! PRESENTER DOES), and clicking a row sets the session's active PAINT tile
//! ([`MapEditorSession::select_tile`]) — the SAME selection the viewport paints with and the save
//! writes.

use bevy_egui::egui;
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    level::UuidThemeRegistry,
    terrain::def::{TerrainDefRegistry, TerrainSimKind, TerrainUuid},
};

use crate::{
    session::MapEditorSession, terrain_graphics::terrain_atlas_index, tile_atlas::TileAtlas,
};

/// The pixel edge of a palette row's sprite thumbnail. A framework layout const.
const THUMB_EDGE: f32 = 24.0;

/// Draw the PREFAB-mode palette into the LEFT panel (GTW-515 C4.2).
///
/// Lists every terrain of the selected theme's palette (sorted by display name for a deterministic
/// order), each as a selectable row `[sprite] name` with the active PAINT tile highlighted; a click
/// selects that tile ([`MapEditorSession::select_tile`]). Below the list, a stat summary for the
/// currently-selected tile (its kind + HP + armor). The sprite thumbnail is an [`egui::Image`] UV
/// sub-rect over the terrain sheet (registered with egui in
/// [`load_tile_atlas`](crate::tile_atlas::load_tile_atlas)); `sheet_id` is that sheet's egui texture
/// id (resolved by the shell before the draw). When the theme/registries have not resolved the
/// panel shows a placeholder rather than an empty list.
pub(crate) fn palette_panel(
    ui: &mut egui::Ui,
    session: &mut MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
    registry: Option<&TerrainDefRegistry>,
    roles: Option<&TileRoles>,
    atlas: Option<&TileAtlas>,
    sheet_id: Option<egui::TextureId>,
) {
    ui.heading("Palette");
    ui.separator();

    let (Some(themes), Some(registry)) = (themes, registry) else {
        ui.label("Loading theme palette…");
        return;
    };
    let theme = session.theme();
    let Some(terrain_keys) = themes.terrain(&theme) else {
        ui.label("Select a theme to see its terrain palette.");
        return;
    };

    // Sort the palette by display name for a deterministic, reader-friendly order (the registry is
    // a HashMap; the theme's terrain list order is authoring order, but we present by name).
    let mut entries: Vec<(TerrainUuid, String)> = terrain_keys
        .iter()
        .filter_map(|key| {
            registry
                .def(key)
                .map(|def| (*key, (*def.display_name).clone()))
        })
        .collect();
    entries.sort_by(|(_, a), (_, b)| a.cmp(b));

    let active = session.selected_tile();
    egui::ScrollArea::vertical()
        .id_salt("prefab_palette_rows")
        .max_height(320.0)
        .show(ui, |ui| {
            for (key, name) in &entries {
                let selected = active == Some(*key);
                if palette_row(ui, name, selected, *key, roles, atlas, registry, sheet_id) {
                    session.select_tile(*key);
                }
            }
        });

    ui.separator();
    selected_tile_stats(ui, session, registry);
}

/// Draw one palette row `[sprite] name` as a selectable region, highlighted when `selected`;
/// returns whether it was clicked. The sprite is the terrain's resolved atlas index drawn as an
/// [`egui::Image`] UV sub-rect over the sheet; a tile with no resolvable index / no atlas / no
/// registered sheet id falls back to just the name (never a panic).
#[expect(
    clippy::too_many_arguments,
    reason = "one palette row draws a sprite thumbnail (needs the tile key + roles + atlas + \
              registry + the egui sheet texture id) beside its selectable name label + the \
              selected flag; each is a borrowed slice threaded from the palette panel"
)]
fn palette_row(
    ui: &mut egui::Ui,
    name: &str,
    selected: bool,
    key: TerrainUuid,
    roles: Option<&TileRoles>,
    atlas: Option<&TileAtlas>,
    registry: &TerrainDefRegistry,
    sheet_id: Option<egui::TextureId>,
) -> bool {
    let response = ui
        .horizontal(|ui| {
            draw_thumb(ui, key, roles, atlas, registry, sheet_id);
            ui.selectable_label(selected, name)
        })
        .inner;
    response.clicked()
}

/// Draw the terrain's sprite thumbnail (GTW-515 C4.2) — an [`egui::Image`] over the sheet's UV
/// sub-rect for the terrain's resolved atlas index. Falls back to a blank fixed-size spacer when
/// the index / atlas / sheet id is unavailable, so the row layout stays stable.
fn draw_thumb(
    ui: &mut egui::Ui,
    key: TerrainUuid,
    roles: Option<&TileRoles>,
    atlas: Option<&TileAtlas>,
    registry: &TerrainDefRegistry,
    sheet_id: Option<egui::TextureId>,
) {
    let resolved = roles
        .zip(atlas)
        .zip(sheet_id)
        .and_then(|((roles, _atlas), id)| {
            terrain_atlas_index(registry, roles, &key).map(|index| (*index, id))
        });
    let Some((index, id)) = resolved else {
        ui.allocate_space(egui::vec2(THUMB_EDGE, THUMB_EDGE));
        return;
    };
    let uv = sheet_uv(index);
    let image = egui::Image::new(egui::load::SizedTexture::new(
        id,
        egui::vec2(THUMB_EDGE, THUMB_EDGE),
    ))
    .uv(uv);
    ui.add(image);
}

/// The UV sub-rect of a terrain sheet atlas `index` — the sheet is a
/// [`SHEET_COLUMNS`](crate::tile_atlas::SHEET_COLUMNS) × [`SHEET_ROWS`](crate::tile_atlas::SHEET_ROWS)
/// grid, so cell `index` sits at column `index % cols`, row `index / cols` and spans one cell in
/// UV space.
fn sheet_uv(index: usize) -> egui::Rect {
    let cols = crate::tile_atlas::SHEET_COLUMNS as usize;
    let rows = crate::tile_atlas::SHEET_ROWS as usize;
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

/// Draw the stat summary for the currently-selected paint tile (GTW-515 C4.2) — its kind + HP +
/// armor, or a placeholder when no tile is selected / the tile is not in the registry.
fn selected_tile_stats(
    ui: &mut egui::Ui,
    session: &MapEditorSession,
    registry: &TerrainDefRegistry,
) {
    ui.label("Selected tile");
    let Some(def) = session.selected_tile().and_then(|key| registry.def(&key)) else {
        ui.label("(none selected)");
        return;
    };
    ui.label(format!("Name: {}", *def.display_name));
    let (kind, hp, protection, hardness) = match &def.sim_kind {
        TerrainSimKind::Wall {
            hp,
            armor_protection,
            armor_hardness,
            ..
        } => ("Wall", **hp, **armor_protection, **armor_hardness),
        TerrainSimKind::Cover {
            hp,
            armor_protection,
            armor_hardness,
            ..
        } => ("Cover", **hp, **armor_protection, **armor_hardness),
        TerrainSimKind::Slab {
            hp,
            armor_protection,
            armor_hardness,
        } => ("Slab", **hp, **armor_protection, **armor_hardness),
    };
    ui.label(format!("Kind: {kind}"));
    ui.label(format!("HP: {hp}"));
    ui.label(format!("Armor: {protection} / hardness {hardness}"));
}
