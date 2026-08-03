use bevy_egui::egui;
use gdtf_battle_sim::ganger::{GangName, GangRegistry};

use crate::gang_form::GangDraft;

pub(crate) fn field_stack(ui: &mut egui::Ui, draft: &mut GangDraft, gangs: Option<&GangRegistry>) {
    ui.heading("Gang");
    ui.separator();

    load_combo(ui, draft, gangs);
    name_field(ui, draft);
    ui.label(format!("Members: {}", draft.members().len()));

    ui.separator();
    if ui.button("New gang").clicked() {
        *draft = GangDraft::new_gang();
    }
    if ui.button("Add member").clicked() {
        draft.add_member();
    }

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_button(ui, draft);
    }
}

fn load_combo(ui: &mut egui::Ui, draft: &mut GangDraft, gangs: Option<&GangRegistry>) {
    ui.label("Load gang");
    let Some(registry) = gangs else {
        ui.label("(loading…)");
        return;
    };
    let mut names: Vec<&GangName> = registry.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let preview = if draft.name().is_empty() {
        "(select…)".to_owned()
    } else {
        draft.name().to_owned()
    };
    let mut chosen: Option<GangName> = None;
    egui::ComboBox::from_id_salt("gang_load_combo")
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
        && let Some(roster) = registry.roster(&name)
    {
        draft.load_gang(&name, roster);
    }
}

fn name_field(ui: &mut egui::Ui, draft: &mut GangDraft) {
    ui.label("Gang name");
    let mut name = draft.name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_name(name);
    }
}

/// `#[cfg(debug_assertions)]` (the terrain / theme save precedent).
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &GangDraft) {
    if !ui.button("Save gang").clicked() {
        return;
    }
    let (name, roster) = crate::gang_form::draft_to_roster(draft);
    match crate::gang_form::write_gang(&name, &roster) {
        Ok(path) => bevy::log::info!(
            "gang save: wrote gang `{}` to `{}`",
            name.as_str(),
            path.display()
        ),
        Err(err) => bevy::log::error!("gang save: gang `{}`: {err}", name.as_str()),
    }
}
