use bevy_egui::egui;
use gdtf_content_families::sprites::{SpriteDefRegistry, SpriteName};

use crate::{save_record::LastSaveRecord, sprite_form::SpriteDraft};

pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut SpriteDraft,
    registry: Option<&SpriteDefRegistry>,
    last_save: &mut LastSaveRecord,
) {
    ui.heading("Sprite");
    ui.separator();

    load_combo(ui, draft, registry);
    name_field(ui, draft);

    ui.separator();
    if ui.button("New sprite").clicked() {
        *draft = SpriteDraft::new_sprite();
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

fn load_combo(ui: &mut egui::Ui, draft: &mut SpriteDraft, registry: Option<&SpriteDefRegistry>) {
    ui.label("Load sprite");
    let Some(registry) = registry else {
        ui.label("(loading…)");
        return;
    };
    let mut names: Vec<&SpriteName> = registry.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let preview = if draft.name().is_empty() {
        "(select…)".to_owned()
    } else {
        draft.name().to_owned()
    };
    let mut chosen: Option<SpriteName> = None;
    egui::ComboBox::from_id_salt("sprite_load_combo")
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
        && let Some(def) = registry.def(&name)
    {
        let def = def.clone();
        draft.load_sprite(&name, &def);
    }
}

fn name_field(ui: &mut egui::Ui, draft: &mut SpriteDraft) {
    ui.label("Sprite name");
    let mut name = draft.name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_name(name);
    }
}

/// `#[cfg(feature = "mcp")]` (the terrain / theme / gang / armor save precedent). The
#[cfg(feature = "mcp")]
fn save_button(ui: &mut egui::Ui, draft: &SpriteDraft, last_save: &mut LastSaveRecord) {
    if !ui.button("Save sprite").clicked() {
        return;
    }
    let (name, def) = crate::sprite_form::draft_to_sprite_def(draft);
    let written = crate::sprite_form::write_sprite(&name, &def);
    match &written {
        Ok(path) => bevy::log::info!(
            "sprite save: wrote sprite `{}` to `{}`",
            name.as_str(),
            path.display()
        ),
        Err(err) => bevy::log::error!("sprite save: sprite `{}`: {err}", name.as_str()),
    }
    last_save.record(
        crate::EditorMode::Sprite,
        crate::save_record::SaveOutcome::from_result(written),
    );
}
