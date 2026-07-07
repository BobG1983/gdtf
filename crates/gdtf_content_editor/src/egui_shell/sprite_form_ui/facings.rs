//! The SPRITE form's **facings section** (GTW-664 C2) — optional per-facing source
//! overrides over the CLOSED 4-facing enum: a checkbox per facing (override on/off)
//! plus, while on, the shared source editor for that facing's override.

use bevy_egui::egui;
use gdtf_content_families::sprites::SpriteFacing;

use super::{cache::SpritePreviewCache, source_edit::source_editor};
use crate::sprite_form::SpriteDraft;

/// The closed facing vocabulary in its canonical N/E/S/W order, with the row labels the
/// section draws.
const FACING_ROWS: [(SpriteFacing, &str); 4] = [
    (SpriteFacing::North, "North"),
    (SpriteFacing::East, "East"),
    (SpriteFacing::South, "South"),
    (SpriteFacing::West, "West"),
];

/// Draw the FACINGS section: one row per [`SpriteFacing`]. Checking a facing seeds its
/// override from the BASE source (the natural starting point the author retargets);
/// unchecking clears it (an emptied map folds back to `facings: None` in the model).
/// Every commit routes through [`SpriteDraft::set_facing_override`]; the checkbox is
/// set-to-target (only a REAL flip writes), so the section is multipass-idempotent
/// (bevy-traps #8).
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
