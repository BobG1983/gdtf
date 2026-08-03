use bevy_egui::egui;
use gdtf_battle_sim::injuries::{InjuryName, InjuryRegistry};

use crate::injury_form::InjuryDraft;

pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut InjuryDraft,
    injuries: Option<&InjuryRegistry>,
) {
    ui.heading("Injury");
    ui.separator();

    load_combo(ui, draft, injuries);
    key_field(ui, draft);

    ui.separator();
    if ui.button("New injury").clicked() {
        *draft = InjuryDraft::new_injury();
    }

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_button(ui, draft);
    }
}

fn load_combo(ui: &mut egui::Ui, draft: &mut InjuryDraft, injuries: Option<&InjuryRegistry>) {
    ui.label("Load injury");
    let Some(registry) = injuries else {
        ui.label("(loading…)");
        return;
    };
    let mut keys: Vec<&InjuryName> = registry.iter().map(|(key, _)| key).collect();
    keys.sort();
    let preview = if draft.key().is_empty() {
        "(select…)".to_owned()
    } else {
        draft.key().to_owned()
    };
    let mut chosen: Option<InjuryName> = None;
    egui::ComboBox::from_id_salt("injury_load_combo")
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
        draft.load_injury(&key, &def);
    }
}

fn key_field(ui: &mut egui::Ui, draft: &mut InjuryDraft) {
    ui.label("Injury key");
    let mut key = draft.key().to_owned();
    if ui.text_edit_singleline(&mut key).changed() {
        draft.set_key(key);
    }
}

/// gated `#[cfg(debug_assertions)]` (the terrain / theme / gang / armor save
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &InjuryDraft) {
    if !ui.button("Save injury").clicked() {
        return;
    }
    let (key, def) = crate::injury_form::draft_to_def(draft);
    match crate::injury_form::write_injury(&key, &def) {
        Ok(path) => bevy::log::info!(
            "injury save: wrote injury `{}` to `{}`",
            key.as_str(),
            path.display()
        ),
        Err(err) => bevy::log::error!("injury save: injury `{}`: {err}", key.as_str()),
    }
}
