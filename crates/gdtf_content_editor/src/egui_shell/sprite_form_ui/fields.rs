//! The SPRITE tab's RIGHT mode-form FIELD STACK (GTW-664 C1) — load an existing def,
//! the sprite-name field, New sprite, and the debug-only Save press (the gang / armor
//! form's `fields.rs` shape).

use bevy_egui::egui;
use gdtf_content_families::sprites::{SpriteDefRegistry, SpriteName};

use crate::sprite_form::SpriteDraft;

/// Draw the SPRITE-mode FIELD STACK into the RIGHT mode-form panel (GTW-664 C1) — the
/// load-sprite [`ComboBox`](egui::ComboBox) (every registry def, sorted), the
/// sprite-name text field (the name IS the file stem / registry key), the New-sprite
/// button, and the debug-only Save button.
///
/// Every control reads / writes the [`SpriteDraft`] through its accessors / mutators;
/// the source / anchor / facings / animation EDITOR lives in the central panel
/// ([`primary_panel`](super::primary_panel)) so the largest space showcases the def.
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

/// The load-sprite [`ComboBox`] — offers EVERY loaded sprite def (sorted by name for a
/// stable order), with the draft's current name as the preview. Choosing a row loads
/// that def into the form through [`SpriteDraft::load_sprite`] (an edit-in-progress is
/// replaced — the load is the author's explicit choice). Falls back to a read-only
/// "(loading…)" label while the registry is absent.
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

/// The sprite-name text field — a single-line edit committed straight into the draft.
/// The name is the registry KEY (= the sanitized file stem on save — the GTW-663
/// stem-key model), so renaming + saving authors a new sprite file.
fn name_field(ui: &mut egui::Ui, draft: &mut SpriteDraft) {
    ui.label("Sprite name");
    let mut name = draft.name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_name(name);
    }
}

/// The debug-only Save button — projects the draft through
/// [`draft_to_sprite_def`](crate::sprite_form::draft_to_sprite_def) (the loader schema,
/// the GTW-636 round-trip contract) and writes it via the one-owner
/// [`write_sprite`](crate::sprite_form::write_sprite) path. On a typed error it logs and
/// writes nothing (never a panic). Debug-only — the whole sprite save path is gated
/// `#[cfg(debug_assertions)]` (the terrain / theme / gang / armor save precedent). The
/// saved member's registry rebuild re-arms the validation pass for free (the
/// [`save`](crate::sprite_form) module doc cites the GTW-663 watch-set edge).
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
