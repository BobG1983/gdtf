//! The SPRITE mode's CENTRAL **primary panel** (GTW-664 C2) — the full def editor,
//! stacked in one scroll area: the base SOURCE picker, the ANCHOR section (drags + the
//! visual crosshair-over-preview affordance), the FACINGS overrides, and the ANIMATION
//! rows.

use bevy_egui::egui;

use super::{
    animation::animation_section,
    cache::SpritePreviewCache,
    facings::facings_section,
    preview::{PreviewTexture, anchor_section},
    source_edit::source_editor,
};
use crate::sprite_form::SpriteDraft;

/// Draw the SPRITE mode's central primary panel over the draft (the injury-mode stacked
/// scroll-area shape). `texture` is the shell-resolved preview of the base source image
/// ([`resolve_preview_texture`](super::resolve_preview_texture) — pre-`ctx_mut`, the
/// `sheet_id` precedent), [`None`] while the image is still decoding / unreadable.
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

/// The base SOURCE section — the shared source editor over the def's base source; a
/// commit routes through [`SpriteDraft::set_base_source`] (which re-clamps the anchor
/// into the new bounds — the model-owned invariant).
fn source_section(ui: &mut egui::Ui, draft: &mut SpriteDraft, cache: &mut SpritePreviewCache) {
    ui.heading("Source");
    let source = draft.def().source.clone();
    if let Some(edited) = source_editor(ui, "base_source", &source, cache) {
        draft.set_base_source(edited);
    }
}
