use bevy_egui::egui;
use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry};

use crate::{armor_form::ArmorDraft, save_record::LastSaveRecord};

pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut ArmorDraft,
    armor: Option<&ArmorRegistry>,
    last_save: &mut LastSaveRecord,
) {
    ui.heading("Armor");
    ui.separator();

    load_combo(ui, draft, armor);
    name_field(ui, draft);

    ui.separator();
    if ui.button("New armor").clicked() {
        *draft = ArmorDraft::new_armor();
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

fn load_combo(ui: &mut egui::Ui, draft: &mut ArmorDraft, armor: Option<&ArmorRegistry>) {
    ui.label("Load armor");
    let Some(registry) = armor else {
        ui.label("(loading…)");
        return;
    };
    let mut names: Vec<&ArmorName> = registry.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let preview = if draft.name().is_empty() {
        "(select…)".to_owned()
    } else {
        draft.name().to_owned()
    };
    let mut chosen: Option<ArmorName> = None;
    egui::ComboBox::from_id_salt("armor_load_combo")
        .selected_text(preview)
        .show_ui(ui, |ui| {
            for name in &names {
                let is_selected = draft.name() == name.as_str();
                if ui.selectable_label(is_selected, name.as_str()).clicked() {
                    chosen = Some((*name).clone());
                }
            }
        });
    if let Some(name) = chosen
        && let Some(spec) = registry.spec(&name)
    {
        let spec = *spec;
        draft.load_armor(&name, &spec);
    }
}

fn name_field(ui: &mut egui::Ui, draft: &mut ArmorDraft) {
    ui.label("Armor name");
    let mut name = draft.name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_name(name);
    }
}

/// `#[cfg(debug_assertions)]` (the terrain / theme / gang save precedent).
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &ArmorDraft, last_save: &mut LastSaveRecord) {
    if !ui.button("Save armor").clicked() {
        return;
    }
    let (name, spec) = crate::armor_form::draft_to_spec(draft);
    let written = crate::armor_form::write_armor(&name, &spec);
    match &written {
        Ok(path) => bevy::log::info!(
            "armor save: wrote armor `{}` to `{}`",
            name.as_str(),
            path.display()
        ),
        Err(err) => bevy::log::error!("armor save: armor `{}`: {err}", name.as_str()),
    }
    last_save.record(
        crate::EditorMode::Armor,
        crate::save_record::SaveOutcome::from_result(written),
    );
}
