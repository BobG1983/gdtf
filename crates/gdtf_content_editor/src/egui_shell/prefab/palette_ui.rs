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
            let def = sprites.and_then(|sprites| terrain_sprite_def(registry, sprites, &key));
            sprite_thumb::draw_thumb(ui, def, textures);
            ui.selectable_label(selected, name)
        })
        .inner;
    response.clicked()
}

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
