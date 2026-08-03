use bevy_egui::egui;
use gdtf_content_families::sprites::{SpriteDefRegistry, SpriteName};

use crate::sprite_form::SpriteDraft;

pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut SpriteDraft,
    registry: Option<&SpriteDefRegistry>,
) {
    ui.heading("Sprite");
    ui.separator();

    load_combo(ui, draft, registry);
    name_field(ui, draft);

    ui.separator();
    if ui.button("New sprite").clicked() {
        *draft = SpriteDraft::new_sprite();
    }

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_button(ui, draft);
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

/// `#[cfg(debug_assertions)]` (the terrain / theme / gang / armor save precedent). The
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &SpriteDraft) {
    if !ui.button("Save sprite").clicked() {
        return;
    }
    let (name, def) = crate::sprite_form::draft_to_sprite_def(draft);
    match crate::sprite_form::write_sprite(&name, &def) {
        Ok(path) => bevy::log::info!(
            "sprite save: wrote sprite `{}` to `{}`",
            name.as_str(),
            path.display()
        ),
        Err(err) => bevy::log::error!("sprite save: sprite `{}`: {err}", name.as_str()),
    }
}
