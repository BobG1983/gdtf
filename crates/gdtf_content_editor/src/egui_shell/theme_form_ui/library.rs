use bevy_egui::egui;
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainUuid};
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::{
    egui_shell::{sprite_thumb, textures::SpriteTextures},
    terrain_graphics::terrain_sprite_def,
    theme_form::{ThemeDraft, sim_kind_label},
};

pub(crate) fn terrain_library_panel(
    ui: &mut egui::Ui,
    draft: &mut ThemeDraft,
    terrain: Option<&TerrainDefRegistry>,
    sprites: Option<&SpriteDefRegistry>,
    textures: &SpriteTextures,
) {
    ui.heading("Terrain library");
    ui.separator();
    let Some(reg) = terrain else {
        ui.label("(loading…)");
        return;
    };
    let mut entries: Vec<(TerrainUuid, String)> = reg
        .defs()
        .map(|(key, def)| {
            let label = format!("{}  [{}]", *def.display_name, sim_kind_label(&def.sim_kind));
            (*key, label)
        })
        .collect();
    entries.sort_by(|(_, a), (_, b)| a.cmp(b));

    egui::ScrollArea::vertical()
        .id_salt("theme_terrain_library")
        .show(ui, |ui| {
            for (key, label) in entries {
                terrain_library_row(ui, draft, reg, sprites, textures, key, &label);
            }
        });
}

fn terrain_library_row(
    ui: &mut egui::Ui,
    draft: &mut ThemeDraft,
    reg: &TerrainDefRegistry,
    sprites: Option<&SpriteDefRegistry>,
    textures: &SpriteTextures,
    key: TerrainUuid,
    label: &str,
) {
    ui.horizontal(|ui| {
        let def = sprites.and_then(|sprites| terrain_sprite_def(reg, sprites, &key));
        sprite_thumb::draw_thumb(ui, def, textures);
        let mut checked = draft.has_terrain(key);
        if ui.checkbox(&mut checked, label).changed() {
            draft.toggle_terrain(key);
        }
    });
}
