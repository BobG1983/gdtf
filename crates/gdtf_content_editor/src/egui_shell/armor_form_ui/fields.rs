//! The ARMOR tab's RIGHT mode-form FIELD STACK (GTW-479 C1) — load an existing armor,
//! the armor-name field, New armor, and the debug-only Save press (the gang form's
//! `fields.rs` shape).

use bevy_egui::egui;
use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry};

use crate::armor_form::ArmorDraft;

/// Draw the ARMOR-mode FIELD STACK into the RIGHT mode-form panel (GTW-479 C1) — the
/// load-armor [`ComboBox`](egui::ComboBox) (every registry suit, sorted), the armor-name
/// text field (the name IS the file stem / registry key), the New-armor button, and the
/// debug-only Save button.
///
/// Every control reads / writes the [`ArmorDraft`] through its accessors / mutators; the
/// per-body-part piece EDITOR lives in the central panel
/// ([`pieces_panel`](super::pieces_panel)) so the largest space showcases the suit.
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

/// The load-armor [`ComboBox`] — offers EVERY loaded armor suit (sorted by name for a
/// stable order), with the draft's current name as the preview. Choosing a row loads
/// that suit's spec into the form through [`ArmorDraft::load_armor`] (an
/// edit-in-progress is replaced — the load is the author's explicit choice). Falls back
/// to a read-only "(loading…)" label while the registry is absent.
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

/// The armor-name text field — a single-line edit committed straight into the draft.
/// The name is the registry KEY (= the sanitized file stem on save — the GTW-269
/// stem-key model), so renaming + saving authors a new armor file.
fn name_field(ui: &mut egui::Ui, draft: &mut ArmorDraft) {
    ui.label("Armor name");
    let mut name = draft.name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_name(name);
    }
}

/// The debug-only Save button — projects the draft through
/// [`draft_to_spec`](crate::armor_form::draft_to_spec) (the loader schema, the GTW-636
/// round-trip contract) and writes it via the one-owner
/// [`write_armor`](crate::armor_form::write_armor) path. On a typed error it logs and
/// writes nothing (never a panic). Debug-only — the whole armor save path is gated
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
