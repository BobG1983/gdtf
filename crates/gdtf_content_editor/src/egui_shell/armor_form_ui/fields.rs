use bevy_egui::egui;
use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry};

use crate::armor_form::ArmorDraft;

pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut ArmorDraft,
    armor: Option<&ArmorRegistry>,
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
        save_button(ui, draft);
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
fn save_button(ui: &mut egui::Ui, draft: &ArmorDraft) {
    if !ui.button("Save armor").clicked() {
        return;
    }
    let (name, spec) = crate::armor_form::draft_to_spec(draft);
    match crate::armor_form::write_armor(&name, &spec) {
        Ok(path) => bevy::log::info!(
            "armor save: wrote armor `{}` to `{}`",
            name.as_str(),
            path.display()
        ),
        Err(err) => bevy::log::error!("armor save: armor `{}`: {err}", name.as_str()),
    }
}
