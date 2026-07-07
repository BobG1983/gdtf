//! The INJURY tab's RIGHT mode-form FIELD STACK (GTW-654 C1) — load an existing
//! injury, the key field, New injury, and the debug-only Save press (the gang /
//! armor forms' `fields.rs` shape).

use bevy_egui::egui;
use gdtf_battle_sim::injuries::{InjuryName, InjuryRegistry};

use crate::injury_form::InjuryDraft;

/// Draw the INJURY-mode FIELD STACK into the RIGHT mode-form panel (GTW-654 C1) —
/// the load-injury [`ComboBox`](egui::ComboBox) (every registry def, sorted by
/// key), the injury-key text field (the key IS the file stem / registry key — the
/// GTW-437 stem-key model), the New-injury button, and the debug-only Save button.
///
/// Every control reads / writes the [`InjuryDraft`] through its accessors /
/// mutators; the def-field + effects-list EDITOR lives in the central panel
/// ([`def_panel`](super::def_panel)) so the largest space showcases the record.
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

/// The load-injury [`ComboBox`] — offers EVERY loaded injury def (sorted by key
/// for a stable order), with the draft's current key as the preview. Choosing a
/// row loads that def into the form through [`InjuryDraft::load_injury`] (an
/// edit-in-progress is replaced — the load is the author's explicit choice). Falls
/// back to a read-only "(loading…)" label while the registry is absent.
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

/// The injury-key text field — a single-line edit committed straight into the
/// draft. The key is the registry KEY (= the sanitized file stem on save, minus
/// the `.injury` infix — the GTW-437 stem-key model), so re-keying + saving
/// authors a new injury file.
fn key_field(ui: &mut egui::Ui, draft: &mut InjuryDraft) {
    ui.label("Injury key");
    let mut key = draft.key().to_owned();
    if ui.text_edit_singleline(&mut key).changed() {
        draft.set_key(key);
    }
}

/// The debug-only Save button — projects the draft through
/// [`draft_to_def`](crate::injury_form::draft_to_def) (the loader schema, the
/// GTW-636 round-trip contract) and writes it via the one-owner
/// [`write_injury`](crate::injury_form::write_injury) path (the target subfolder
/// derives from the def's own authored category). On a typed error it logs and
/// writes nothing (never a panic). Debug-only — the whole injury save path is
/// gated `#[cfg(debug_assertions)]` (the terrain / theme / gang / armor save
/// precedent).
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
