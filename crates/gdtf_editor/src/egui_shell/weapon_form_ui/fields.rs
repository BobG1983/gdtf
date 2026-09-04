use bevy_egui::egui;
use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry};

use crate::{save_record::LastSaveRecord, weapon_form::WeaponDraft};

pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut WeaponDraft,
    registry: Option<&WeaponRegistry>,
    last_save: &mut LastSaveRecord,
) {
    ui.heading("Weapon");
    ui.separator();

    load_combo(ui, draft, registry);
    name_field(ui, draft);

    ui.separator();
    if ui.button("New weapon").clicked() {
        *draft = WeaponDraft::new_weapon();
    }

    #[cfg(feature = "mcp")]
    {
        ui.separator();
        save_button(ui, draft, last_save);
    }
    #[cfg(not(feature = "mcp"))]
    {
        let _ = last_save;
    }
}

fn load_combo(ui: &mut egui::Ui, draft: &mut WeaponDraft, registry: Option<&WeaponRegistry>) {
    ui.label("Load weapon");
    let Some(registry) = registry else {
        ui.label("(loading…)");
        return;
    };
    let mut names: Vec<&WeaponName> = registry.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let preview = if draft.name().is_empty() {
        "(select…)".to_owned()
    } else {
        draft.name().to_owned()
    };
    let mut chosen: Option<WeaponName> = None;
    egui::ComboBox::from_id_salt("weapon_load_combo")
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
        let spec = spec.clone();
        draft.load_weapon(&name, &spec);
    }
}

fn name_field(ui: &mut egui::Ui, draft: &mut WeaponDraft) {
    ui.label("Weapon name");
    let mut name = draft.name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_name(name);
    }
}

/// gated `#[cfg(feature = "mcp")]` (the terrain / theme / gang / armor / sprite /
#[cfg(feature = "mcp")]
fn save_button(ui: &mut egui::Ui, draft: &WeaponDraft, last_save: &mut LastSaveRecord) {
    if !ui.button("Save weapon").clicked() {
        return;
    }
    let (name, spec) = crate::weapon_form::draft_to_weapon_spec(draft);
    let written = crate::weapon_form::write_weapon(&name, &spec);
    match &written {
        Ok(path) => bevy::log::info!(
            "weapon save: wrote weapon `{}` to `{}`",
            name.as_str(),
            path.display()
        ),
        Err(err) => bevy::log::error!("weapon save: weapon `{}`: {err}", name.as_str()),
    }
    last_save.record(
        crate::EditorMode::Weapon,
        crate::save_record::SaveOutcome::from_result(written),
    );
}
