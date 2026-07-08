//! The ATTACHMENT tab's RIGHT mode-form FIELD STACK (GTW-669 C2) — load an existing
//! item, the item-name field, New attachment, and the debug-only Save press (the gang /
//! armor / sprite form's `fields.rs` shape).

use bevy_egui::egui;
use gdtf_battle_sim::equipment::attachments::{AttachmentName, AttachmentRegistry};

use crate::attachment_form::AttachmentDraft;

/// Draw the ATTACHMENT-mode FIELD STACK into the RIGHT mode-form panel (GTW-669 C2) —
/// the load-attachment [`ComboBox`](egui::ComboBox) (every registry item, sorted), the
/// item-name text field (the name IS the file stem / registry key — the GTW-549
/// stem-key model), the New-attachment button, and the debug-only Save button.
///
/// Every control reads / writes the [`AttachmentDraft`] through its accessors /
/// mutators; the display-name / slot / effects-list EDITOR lives in the central panel
/// ([`def_panel`](super::def_panel)) so the largest space showcases the item.
pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut AttachmentDraft,
    registry: Option<&AttachmentRegistry>,
) {
    ui.heading("Attachment");
    ui.separator();

    load_combo(ui, draft, registry);
    name_field(ui, draft);

    ui.separator();
    if ui.button("New attachment").clicked() {
        *draft = AttachmentDraft::new_attachment();
    }

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_button(ui, draft);
    }
}

/// The load-attachment [`ComboBox`] — offers EVERY loaded item (sorted by key for a
/// stable order), with the draft's current name as the preview. Choosing a row loads
/// that item into the form through [`AttachmentDraft::load_attachment`] (an
/// edit-in-progress is replaced — the load is the author's explicit choice). Falls back
/// to a read-only "(loading…)" label while the registry is absent.
fn load_combo(
    ui: &mut egui::Ui,
    draft: &mut AttachmentDraft,
    registry: Option<&AttachmentRegistry>,
) {
    ui.label("Load attachment");
    let Some(registry) = registry else {
        ui.label("(loading…)");
        return;
    };
    let mut names: Vec<&AttachmentName> = registry.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let preview = if draft.name().is_empty() {
        "(select…)".to_owned()
    } else {
        draft.name().to_owned()
    };
    let mut chosen: Option<AttachmentName> = None;
    egui::ComboBox::from_id_salt("attachment_load_combo")
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
        draft.load_attachment(&name, &spec);
    }
}

/// The item-name text field — a single-line edit committed straight into the draft. The
/// name is the registry KEY (= the sanitized file stem on save — the GTW-549 stem-key
/// model), so renaming + saving authors a new attachment file.
fn name_field(ui: &mut egui::Ui, draft: &mut AttachmentDraft) {
    ui.label("Attachment name");
    let mut name = draft.name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_name(name);
    }
}

/// The debug-only Save button — projects the draft through
/// [`draft_to_attachment_spec`](crate::attachment_form::draft_to_attachment_spec) (the
/// loader schema, the GTW-636 round-trip contract) and writes it via the one-owner
/// [`write_attachment`](crate::attachment_form::write_attachment) path. On a typed error
/// it logs and writes nothing (never a panic). Debug-only — the whole attachment save
/// path is gated `#[cfg(debug_assertions)]` (the terrain / theme / gang / armor / sprite
/// save precedent). The saved member's registry rebuild re-arms the validation pass for
/// free (the [`save`](crate::attachment_form) module doc cites the GTW-669 watch-set
/// edge).
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &AttachmentDraft) {
    if !ui.button("Save attachment").clicked() {
        return;
    }
    let (name, spec) = crate::attachment_form::draft_to_attachment_spec(draft);
    match crate::attachment_form::write_attachment(&name, &spec) {
        Ok(path) => bevy::log::info!(
            "attachment save: wrote attachment `{}` to `{}`",
            name.as_str(),
            path.display()
        ),
        Err(err) => bevy::log::error!("attachment save: attachment `{}`: {err}", name.as_str()),
    }
}
