//! The GANG tab's RIGHT mode-form FIELD STACK (GTW-636 C1) — load an existing gang,
//! the gang-name field, New gang / Add member, the member count, and the debug-only
//! Save press (the theme form's `fields.rs` shape).

use bevy_egui::egui;
use gdtf_battle_sim::ganger::{GangName, GangRegistry};

use crate::gang_form::GangDraft;

/// Draw the GANG-mode FIELD STACK into the RIGHT mode-form panel (GTW-636 C1) — the
/// load-gang [`ComboBox`](egui::ComboBox) (every registry gang, sorted), the gang-name
/// text field (the name IS the file stem / registry key), the New-gang and Add-member
/// buttons, a member-count line, and the debug-only Save button.
///
/// Every control reads / writes the [`GangDraft`] through its accessors / mutators; the
/// member EDITORS live in the central panel
/// ([`members_panel`](super::members_panel)) so the largest space showcases the roster.
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

/// The load-gang [`ComboBox`] — offers EVERY loaded gang (sorted by name for a stable
/// order), with the draft's current name as the preview. Choosing a row loads that
/// gang's roster into the form through [`GangDraft::load_gang`] (an edit-in-progress is
/// replaced — the load is the author's explicit choice). Falls back to a read-only
/// "(loading…)" label while the registry is absent.
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

/// The gang-name text field — a single-line edit committed straight into the draft.
/// The name is the registry KEY (= the sanitized file stem on save — the GTW-415 key
/// model), so renaming + saving authors a new gang file.
fn name_field(ui: &mut egui::Ui, draft: &mut GangDraft) {
    ui.label("Gang name");
    let mut name = draft.name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_name(name);
    }
}

/// The debug-only Save button — projects the draft through
/// [`draft_to_roster`](crate::gang_form::draft_to_roster) (the loader schema, the
/// GTW-429 round-trip contract) and writes it via the one-owner
/// [`write_gang`](crate::gang_form::write_gang) path. On a typed error it logs and
/// writes nothing (never a panic). Debug-only — the whole gang save path is gated
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
