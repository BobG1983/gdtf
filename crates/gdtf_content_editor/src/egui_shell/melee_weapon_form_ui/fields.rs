//! The MELEE tab's RIGHT mode-form FIELD STACK (GTW-671 C1) — load an existing melee
//! weapon, the weapon-name field, New melee weapon, and the debug-only Save press (the
//! gang / armor / sprite / attachment / weapon form's `fields.rs` shape).

use bevy_egui::egui;
use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, WeaponName};

use crate::melee_weapon_form::MeleeWeaponDraft;

/// Draw the MELEE-WEAPON-mode FIELD STACK into the RIGHT mode-form panel (GTW-671 C1)
/// — the load-weapon [`ComboBox`](egui::ComboBox) (every registry melee weapon,
/// sorted), the weapon-name text field (the name IS the file stem / registry key — the
/// GTW-257 stem-key model the melee family shares), the New-melee-weapon button, and
/// the debug-only Save button.
///
/// Every control reads / writes the [`MeleeWeaponDraft`] through its accessors /
/// mutators; the full spec EDITOR lives in the central panel
/// ([`def_panel`](super::def_panel)) so the largest space hosts the long form.
pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut MeleeWeaponDraft,
    registry: Option<&MeleeWeaponRegistry>,
) {
    ui.heading("Melee weapon");
    ui.separator();

    load_combo(ui, draft, registry);
    name_field(ui, draft);

    ui.separator();
    if ui.button("New melee weapon").clicked() {
        *draft = MeleeWeaponDraft::new_melee_weapon();
    }

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_button(ui, draft);
    }
}

/// The load-weapon [`ComboBox`] — offers EVERY loaded melee weapon (sorted by key for
/// a stable order), with the draft's current name as the preview. Choosing a row loads
/// that weapon into the form through [`MeleeWeaponDraft::load_melee_weapon`] (an
/// edit-in-progress is replaced — the load is the author's explicit choice). Falls
/// back to a read-only "(loading…)" label while the registry is absent.
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

/// The weapon-name text field — a single-line edit committed straight into the draft.
/// The name is the registry KEY (= the sanitized file stem on save — the GTW-257
/// stem-key model), so renaming + saving authors a new melee-weapon file.
fn name_field(ui: &mut egui::Ui, draft: &mut MeleeWeaponDraft) {
    ui.label("Melee weapon name");
    let mut name = draft.name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_name(name);
    }
}

/// The debug-only Save button — projects the draft through
/// [`draft_to_melee_weapon_spec`](crate::melee_weapon_form::draft_to_melee_weapon_spec)
/// (the loader schema, the GTW-636 round-trip contract) and writes it via the one-owner
/// [`write_melee_weapon`](crate::melee_weapon_form::write_melee_weapon) path. On a
/// typed error it logs and writes nothing (never a panic). Debug-only — the whole
/// melee-weapon save path is gated `#[cfg(debug_assertions)]` (the terrain / theme /
/// gang / armor / sprite / attachment / weapon save precedent). The saved member's
/// registry rebuild re-arms the validation pass for free (the
/// [`save`](crate::melee_weapon_form) module doc cites the
/// `WatchedRegistries::melee_weapons` watch-set edge).
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &MeleeWeaponDraft) {
    if !ui.button("Save melee weapon").clicked() {
        return;
    }
    let (name, spec) = crate::melee_weapon_form::draft_to_melee_weapon_spec(draft);
    match crate::melee_weapon_form::write_melee_weapon(&name, &spec) {
        Ok(path) => bevy::log::info!(
            "melee weapon save: wrote melee weapon `{}` to `{}`",
            name.as_str(),
            path.display()
        ),
        Err(err) => bevy::log::error!("melee weapon save: melee weapon `{}`: {err}", name.as_str()),
    }
}
