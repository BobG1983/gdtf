use bevy_egui::egui;

use super::{
    animation::animation_section,
    cache::SpritePreviewCache,
    facings::facings_section,
    preview::{PreviewTexture, anchor_section},
    source_edit::source_editor,
};
use crate::sprite_form::SpriteDraft;

pub(crate) fn primary_panel(
    ui: &mut egui::Ui,
    draft: &mut SpriteDraft,
    cache: &mut SpritePreviewCache,
    texture: Option<&PreviewTexture>,
) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            source_section(ui, draft, cache);
            ui.separator();
            anchor_section(ui, draft, texture);
            ui.separator();
            facings_section(ui, draft, cache);
            ui.separator();
            animation_section(ui, draft, cache);
        });
}

fn source_section(ui: &mut egui::Ui, draft: &mut SpriteDraft, cache: &mut SpritePreviewCache) {
    ui.heading("Source");
    let source = draft.def().source.clone();
    if let Some(edited) = source_editor(ui, "base_source", &source, cache) {
        draft.set_base_source(edited);
    }
}
