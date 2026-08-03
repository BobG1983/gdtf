use bevy_egui::egui;
use gdtf_battle_presenter::{TileRole, resolve_sprite};
use gdtf_battle_sim::{level::UuidThemeRegistry, weapon::WeaponRegistry};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::fields::field_stack;
use crate::{
    egui_shell::{
        sprite_thumb::{self, THUMB_EDGE},
        textures::SpriteTextures,
    },
    session::MapEditorSession,
    terrain_form::{TerrainDraft, offered_graphic_roles},
};

const PICKER_COLUMNS: usize = 5;

pub(crate) fn primary_panel(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
    sprites: Option<&SpriteDefRegistry>,
    weapons: Option<&WeaponRegistry>,
    textures: &SpriteTextures,
) {
    ui.columns(2, |columns| {
        if let [picker_col, fields_col] = columns {
            graphic_picker(picker_col, draft, sprites, textures);
            field_stack(fields_col, draft, session, themes, weapons);
        }
    });
}

fn graphic_picker(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    sprites: Option<&SpriteDefRegistry>,
    textures: &SpriteTextures,
) {
    ui.heading("Graphic role");
    ui.separator();
    let active = draft.graphic();
    let mut picked: Option<TileRole> = None;
    egui::Grid::new("terrain_graphic_picker_grid")
        .spacing(egui::vec2(4.0, 4.0))
        .show(ui, |ui| {
            for (slot, choice) in offered_graphic_roles().into_iter().enumerate() {
                if graphic_cell(ui, choice, choice == active, sprites, textures) {
                    picked = Some(choice);
                }
                if (slot + 1) % PICKER_COLUMNS == 0 {
                    ui.end_row();
                }
            }
        });
    if let Some(choice) = picked {
        draft.set_graphic(choice);
    }
}

fn graphic_cell(
    ui: &mut egui::Ui,
    choice: TileRole,
    selected: bool,
    sprites: Option<&SpriteDefRegistry>,
    textures: &SpriteTextures,
) -> bool {
    let def = sprites.and_then(|sprites| resolve_sprite(sprites, choice.as_key()));
    let button = match sprite_thumb::thumb_image(def, textures) {
        Some(image) => egui::Button::image(image).selected(selected).frame(true),
        None => egui::Button::new(choice.as_key())
            .selected(selected)
            .min_size(egui::vec2(THUMB_EDGE, THUMB_EDGE)),
    };
    ui.add(button).on_hover_text(choice.as_key()).clicked()
}
