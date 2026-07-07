//! The PREFAB-mode **palette panel** (GTW-515 C4.2) — the LEFT panel: one selectable row per
//! terrain in the selected theme's palette, each showing its sprite thumbnail (via
//! [`terrain_sprite_def`]) with the active row highlighted, plus a stat summary for the selected
//! tile.
//!
//! Reuses the theme / terrain / sprite-def registries + the presenter's [`terrain_sprite_def`]
//! resolution VERBATIM (C4.2 / GTW-665): a row's sprite is the terrain's own graphic (resolved
//! THE WAY THE
//! PRESENTER DOES), and clicking a row sets the session's active PAINT tile
//! ([`MapEditorSession::select_tile`]) — the SAME selection the viewport paints with and the save
//! writes.

use bevy_egui::egui;
use gdtf_battle_sim::{
    level::UuidThemeRegistry,
    terrain::def::{TerrainDefRegistry, TerrainSimKind, TerrainUuid},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::{
    egui_shell::{sprite_thumb, textures::SpriteTextures},
    session::MapEditorSession,
    terrain_graphics::terrain_sprite_def,
};

/// Draw the PREFAB-mode palette into the LEFT panel (GTW-515 C4.2).
///
/// Lists every terrain of the selected theme's palette (sorted by display name for a deterministic
/// order), each as a selectable row `[sprite] name` with the active PAINT tile highlighted; a click
/// selects that tile ([`MapEditorSession::select_tile`]). Below the list, a stat summary for the
/// currently-selected tile (its kind + HP + armor). The sprite thumbnail is an [`egui::Image`] UV
/// sub-rect over the def's source texture (the shell-resolved per-path [`SpriteTextures`] map —
/// GTW-665). When the theme/registries have not resolved the
/// panel shows a placeholder rather than an empty list.
pub(in crate::egui_shell) fn palette_panel(
    ui: &mut egui::Ui,
    session: &mut MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
    registry: Option<&TerrainDefRegistry>,
    sprites: Option<&SpriteDefRegistry>,
    textures: &SpriteTextures,
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
                if palette_row(ui, name, selected, *key, sprites, registry, textures) {
                    session.select_tile(*key);
                }
            }
        });

    ui.separator();
    selected_tile_stats(ui, session, registry);
}

/// Draw one palette row `[sprite] name` as a selectable region, highlighted when `selected`;
/// returns whether it was clicked. The sprite is the terrain's resolved sprite def drawn as an
/// [`egui::Image`] UV sub-rect over its source texture; a def-less graphic paints the loud
/// magenta missing square, a still-decoding source a fixed-size spacer (never a panic).
fn palette_row(
    ui: &mut egui::Ui,
    name: &str,
    selected: bool,
    key: TerrainUuid,
    sprites: Option<&SpriteDefRegistry>,
    registry: &TerrainDefRegistry,
    textures: &SpriteTextures,
) -> bool {
    let response = ui
        .horizontal(|ui| {
            // Resolve THE WAY THE PRESENTER DOES (GTW-665 — terrain_sprite_def) and hand
            // the def to the SHARED thumbnail draw (GTW-516 C3).
            let def = sprites.and_then(|sprites| terrain_sprite_def(registry, sprites, &key));
            sprite_thumb::draw_thumb(ui, def, textures);
            ui.selectable_label(selected, name)
        })
        .inner;
    response.clicked()
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
        // GTW-543: an emplacement shows its structural stats like a Wall/Cover (the
        // mounted-weapon key is not shown in this compact readout).
        TerrainSimKind::Emplacement {
            hp,
            armor_protection,
            armor_hardness,
            ..
        } => ("Emplacement", **hp, **armor_protection, **armor_hardness),
    };
    ui.label(format!("Kind: {kind}"));
    ui.label(format!("HP: {hp}"));
    ui.label(format!("Armor: {protection} / hardness {hardness}"));
}
