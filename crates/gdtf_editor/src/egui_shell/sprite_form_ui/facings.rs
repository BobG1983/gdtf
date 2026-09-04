use bevy_egui::egui;
use gdtf_content_families::sprites::SpriteFacing;

use super::{cache::SpritePreviewCache, source_edit::source_editor};
use crate::sprite_form::SpriteDraft;

const FACING_ROWS: [(SpriteFacing, &str); 4] = [
    (SpriteFacing::North, "North"),
    (SpriteFacing::East, "East"),
    (SpriteFacing::South, "South"),
    (SpriteFacing::West, "West"),
];

pub(super) fn facings_section(
    ui: &mut egui::Ui,
    draft: &mut SpriteDraft,
    cache: &mut SpritePreviewCache,
) {
    ui.heading("Facings");
    ui.label("Optional per-facing source overrides — an unchecked facing draws the base source.");
    for (facing, label) in FACING_ROWS {
        let current = draft.facing_override(facing).cloned();
        let mut enabled = current.is_some();
        if ui.checkbox(&mut enabled, label).changed() {
            let seed = enabled.then(|| draft.def().source.clone());
            draft.set_facing_override(facing, seed);
        }
        if let Some(source) = draft.facing_override(facing).cloned() {
            ui.indent(format!("facing_{label}_indent"), |ui| {
                if let Some(edited) = source_editor(ui, &format!("facing_{label}"), &source, cache)
                {
                    draft.set_facing_override(facing, Some(edited));
                }
            });
        }
    }
}
