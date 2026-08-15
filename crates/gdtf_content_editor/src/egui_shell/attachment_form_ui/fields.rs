use bevy_egui::egui;
use gdtf_battle_sim::equipment::attachments::{AttachmentName, AttachmentRegistry};

use crate::{attachment_form::AttachmentDraft, save_record::LastSaveRecord};

pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut AttachmentDraft,
    registry: Option<&AttachmentRegistry>,
    last_save: &mut LastSaveRecord,
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
        save_button(ui, draft, last_save);
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = last_save;
    }
}

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

fn name_field(ui: &mut egui::Ui, draft: &mut AttachmentDraft) {
    ui.label("Attachment name");
    let mut name = draft.name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_name(name);
    }
}

/// path is gated `#[cfg(debug_assertions)]` (the terrain / theme / gang / armor / sprite
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &AttachmentDraft, last_save: &mut LastSaveRecord) {
    if !ui.button("Save attachment").clicked() {
        return;
    }
    let (name, spec) = crate::attachment_form::draft_to_attachment_spec(draft);
    let written = crate::attachment_form::write_attachment(&name, &spec);
    match &written {
        Ok(path) => bevy::log::info!(
            "attachment save: wrote attachment `{}` to `{}`",
            name.as_str(),
            path.display()
        ),
        Err(err) => bevy::log::error!("attachment save: attachment `{}`: {err}", name.as_str()),
    }
    last_save.record(
        crate::EditorMode::Attachment,
        crate::save_record::SaveOutcome::from_result(written),
    );
}
