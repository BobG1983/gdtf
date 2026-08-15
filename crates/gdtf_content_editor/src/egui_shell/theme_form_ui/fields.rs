use bevy_egui::egui;
use gdtf_battle_sim::terrain::def::TerrainDefRegistry;

use crate::{
    save_record::LastSaveRecord,
    theme_form::{ThemeDraft, slab_floor_candidates},
};

const KEY_PREFIX: &str = "UUID: ";

pub(in crate::egui_shell) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut ThemeDraft,
    terrain: Option<&TerrainDefRegistry>,
    last_save: &mut LastSaveRecord,
) {
    ui.heading("Theme");
    ui.separator();

    name_field(ui, draft);
    floor_combo(ui, draft, terrain);
    uuid_text(ui, draft);

    ui.separator();
    new_theme_button(ui, draft);

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_button(ui, draft, last_save);
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = last_save;
    }
}

fn name_field(ui: &mut egui::Ui, draft: &mut ThemeDraft) {
    ui.label("Display name");
    let mut name = draft.display_name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_display_name(name);
    }
}

fn floor_combo(ui: &mut egui::Ui, draft: &mut ThemeDraft, terrain: Option<&TerrainDefRegistry>) {
    ui.label("Default floor");
    let Some(reg) = terrain else {
        ui.label("(loading…)");
        return;
    };
    let candidates = slab_floor_candidates(draft, reg);
    if candidates.is_empty() {
        ui.add_enabled_ui(false, |ui| {
            egui::ComboBox::from_id_salt("theme_floor_combo")
                .selected_text("(none)")
                .show_ui(ui, |_ui| {});
        });
        return;
    }
    let current_floor = draft.default_floor();
    let preview = current_floor
        .and_then(|key| candidates.iter().find(|(k, _)| *k == key))
        .map_or_else(|| "(select…)".to_owned(), |(_, label)| label.clone());
    let mut chosen = current_floor;
    let mut clicked = false;
    egui::ComboBox::from_id_salt("theme_floor_combo")
        .selected_text(preview)
        .show_ui(ui, |ui| {
            for (key, label) in &candidates {
                let is_selected = Some(*key) == current_floor;
                if ui.selectable_label(is_selected, label).clicked() {
                    chosen = Some(*key);
                    clicked = true;
                }
            }
        });
    if clicked && let Some(key) = chosen {
        draft.set_default_floor(key);
    }
}

fn uuid_text(ui: &mut egui::Ui, draft: &ThemeDraft) {
    let key_str = format!("{}{}", KEY_PREFIX, *draft.key());
    ui.label(key_str);
}

fn new_theme_button(ui: &mut egui::Ui, draft: &mut ThemeDraft) {
    if ui.button("New theme").clicked() {
        *draft = ThemeDraft::new_theme();
    }
}

/// `#[cfg(debug_assertions)]`.
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &ThemeDraft, last_save: &mut LastSaveRecord) {
    if !ui.button("Save theme").clicked() {
        return;
    }
    let key = draft.key();
    let written = crate::theme_form::write_theme(draft, key);
    match &written {
        Ok(path) => bevy::log::info!("theme save: wrote theme def to `{}`", path.display()),
        Err(err) => bevy::log::error!("theme save: {err}"),
    }
    last_save.record(
        crate::EditorMode::Theme,
        crate::save_record::SaveOutcome::from_result(written),
    );
}
