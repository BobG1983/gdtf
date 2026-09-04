use bevy_egui::egui;
use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, WeaponName};

use crate::{melee_weapon_form::MeleeWeaponDraft, save_record::LastSaveRecord};

pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut MeleeWeaponDraft,
    registry: Option<&MeleeWeaponRegistry>,
    last_save: &mut LastSaveRecord,
) {
    ui.heading("Melee weapon");
    ui.separator();

    load_combo(ui, draft, registry);
    name_field(ui, draft);

    ui.separator();
    if ui.button("New melee weapon").clicked() {
        *draft = MeleeWeaponDraft::new_melee_weapon();
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

fn load_combo(
    ui: &mut egui::Ui,
    draft: &mut MeleeWeaponDraft,
    registry: Option<&MeleeWeaponRegistry>,
) {
    ui.label("Load melee weapon");
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
    egui::ComboBox::from_id_salt("melee_weapon_load_combo")
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
        draft.load_melee_weapon(&name, &spec);
    }
}

fn name_field(ui: &mut egui::Ui, draft: &mut MeleeWeaponDraft) {
    ui.label("Melee weapon name");
    let mut name = draft.name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_name(name);
    }
}

/// melee-weapon save path is gated `#[cfg(feature = "mcp")]` (the terrain / theme /
#[cfg(feature = "mcp")]
fn save_button(ui: &mut egui::Ui, draft: &MeleeWeaponDraft, last_save: &mut LastSaveRecord) {
    if !ui.button("Save melee weapon").clicked() {
        return;
    }
    let (name, spec) = crate::melee_weapon_form::draft_to_melee_weapon_spec(draft);
    let written = crate::melee_weapon_form::write_melee_weapon(&name, &spec);
    match &written {
        Ok(path) => bevy::log::info!(
            "melee weapon save: wrote melee weapon `{}` to `{}`",
            name.as_str(),
            path.display()
        ),
        Err(err) => bevy::log::error!("melee weapon save: melee weapon `{}`: {err}", name.as_str()),
    }
    last_save.record(
        crate::EditorMode::MeleeWeapon,
        crate::save_record::SaveOutcome::from_result(written),
    );
}
