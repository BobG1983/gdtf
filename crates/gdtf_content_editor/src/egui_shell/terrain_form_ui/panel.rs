use bevy_egui::egui;
use gdtf_battle_presenter::resolve_sprite;
use gdtf_battle_sim::{
    terrain::{
        def::{TerrainDefRegistry, TerrainView},
        piece::TerrainGraphicKey,
    },
    weapon::WeaponRegistry,
};
use gdtf_content_families::sprites::{SpriteDefRegistry, SpriteName};

use super::fields::{TerrainSaveContext, field_stack};
use crate::{
    egui_shell::{
        sprite_thumb::{self, THUMB_EDGE},
        textures::SpriteTextures,
    },
    terrain_form::{TerrainDraft, offers_view_expander, view_rows},
};

const PICKER_COLUMNS: usize = 5;

pub(in crate::egui_shell) fn primary_panel(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    save: TerrainSaveContext<'_>,
    sprites: Option<&SpriteDefRegistry>,
    weapons: Option<&WeaponRegistry>,
    terrain: Option<&TerrainDefRegistry>,
    textures: &SpriteTextures,
) {
    ui.columns(2, |columns| {
        if let [picker_col, fields_col] = columns {
            graphic_picker(picker_col, draft, sprites, textures);
            field_stack(fields_col, draft, save, weapons, terrain, sprites);
        }
    });
}

// Every sprite the grid offers, sorted by key so the cells sit still across runs.
fn offered_sprites(sprites: Option<&SpriteDefRegistry>) -> Vec<&SpriteName> {
    let mut keys: Vec<&SpriteName> = sprites
        .map(|sprites| sprites.keys().collect())
        .unwrap_or_default();
    keys.sort_by_key(|key| &***key);
    keys
}

fn graphic_picker(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    sprites: Option<&SpriteDefRegistry>,
    textures: &SpriteTextures,
) {
    ui.heading("Views");
    ui.separator();
    let rows = view_rows(draft);
    let keys = offered_sprites(sprites);
    let mut picked: Option<(TerrainView, TerrainGraphicKey)> = None;
    if offers_view_expander(draft) {
        egui::CollapsingHeader::new("Owed views")
            .default_open(true)
            .show(ui, |ui| {
                picked = view_row_stack(ui, draft, &rows, &keys, sprites, textures);
            });
    } else {
        picked = view_row_stack(ui, draft, &rows, &keys, sprites, textures);
    }
    if let Some((view, key)) = picked {
        draft.set_view(view, key);
    }
}

// One labelled row per view the draft owes, each row a grid of every sprite key.
fn view_row_stack(
    ui: &mut egui::Ui,
    draft: &TerrainDraft,
    rows: &[TerrainView],
    keys: &[&SpriteName],
    sprites: Option<&SpriteDefRegistry>,
    textures: &SpriteTextures,
) -> Option<(TerrainView, TerrainGraphicKey)> {
    let mut picked: Option<(TerrainView, TerrainGraphicKey)> = None;
    for view in rows {
        ui.label(format!("{view:?}"));
        let held = draft.views().sprite(*view).map(|key| (**key).clone());
        egui::Grid::new(format!("terrain_view_picker_grid_{view:?}"))
            .spacing(egui::vec2(4.0, 4.0))
            .show(ui, |ui| {
                for (slot, key) in keys.iter().enumerate() {
                    let selected = held.as_deref() == Some(&***key);
                    if graphic_cell(ui, key, selected, sprites, textures) {
                        picked = Some((*view, TerrainGraphicKey::new((***key).clone())));
                    }
                    if (slot + 1) % PICKER_COLUMNS == 0 {
                        ui.end_row();
                    }
                }
            });
    }
    picked
}

fn graphic_cell(
    ui: &mut egui::Ui,
    key: &SpriteName,
    selected: bool,
    sprites: Option<&SpriteDefRegistry>,
    textures: &SpriteTextures,
) -> bool {
    let def = sprites.and_then(|sprites| resolve_sprite(sprites, key));
    let button = match sprite_thumb::thumb_image(def, textures) {
        Some(image) => egui::Button::image(image).selected(selected).frame(true),
        None => egui::Button::new(&***key)
            .selected(selected)
            .min_size(egui::vec2(THUMB_EDGE, THUMB_EDGE)),
    };
    ui.add(button).on_hover_text(&***key).clicked()
}
