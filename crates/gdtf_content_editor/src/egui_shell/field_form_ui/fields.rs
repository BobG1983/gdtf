use bevy_egui::egui;
use gdtf_battle_sim::effects::fields::{FieldDefRegistry, FieldKey};

use crate::{field_form::FieldDraft, save_record::LastSaveRecord};

pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut FieldDraft,
    registry: Option<&FieldDefRegistry>,
    last_save: &mut LastSaveRecord,
) {
    ui.heading("Field");
    ui.separator();

    load_combo(ui, draft, registry);
    key_field(ui, draft);

    ui.separator();
    if ui.button("New field").clicked() {
        *draft = FieldDraft::new_field();
    }

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

fn load_combo(ui: &mut egui::Ui, draft: &mut FieldDraft, registry: Option<&FieldDefRegistry>) {
    ui.label("Load field");
    let Some(registry) = registry else {
        ui.label("(loading…)");
        return;
    };
    let mut keys: Vec<&FieldKey> = registry.keys().collect();
    keys.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let preview = if draft.key().is_empty() {
        "(select…)".to_owned()
    } else {
        draft.key().to_owned()
    };
    let mut chosen: Option<FieldKey> = None;
    egui::ComboBox::from_id_salt("field_load_combo")
        .selected_text(preview)
        .show_ui(ui, |ui| {
            for key in &keys {
                let is_selected = draft.key() == key.as_str();
                if ui.selectable_label(is_selected, key.as_str()).clicked() {
                    chosen = Some((*key).clone());
                }
            }
        });
    if let Some(key) = chosen
        && let Some(def) = registry.def(&key)
    {
        let def = def.clone();
        draft.load_field(&key, &def);
    }
}

fn key_field(ui: &mut egui::Ui, draft: &mut FieldDraft) {
    ui.label("Field key");
    let mut key = draft.key().to_owned();
    if ui.text_edit_singleline(&mut key).changed() {
        draft.set_key(key);
    }
}

/// `#[cfg(debug_assertions)]`, the save precedent every other form's button follows.
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &FieldDraft, last_save: &mut LastSaveRecord) {
    if !ui.button("Save field").clicked() {
        return;
    }
    let (key, def) = crate::field_form::draft_to_field(draft);
    let written = crate::field_form::write_field(&key, &def);
    match &written {
        Ok(path) => bevy::log::info!(
            "field save: wrote field `{}` to `{}`",
            key.as_str(),
            path.display()
        ),
        Err(err) => bevy::log::error!("field save: field `{}`: {err}", key.as_str()),
    }
    last_save.record(
        crate::EditorMode::Field,
        crate::save_record::SaveOutcome::from_result(written),
    );
}
