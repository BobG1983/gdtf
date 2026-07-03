//! The TERRAIN tab's demoted `.terrain_def.ron` live preview (GTW-534 C2).

use bevy_egui::egui;
use gdtf_battle_sim::terrain::def::TerrainUuid;

use crate::terrain_form::{TerrainDraft, draft_to_terrain_def, serialize_terrain_def};

/// Draw the live monospace `.terrain_def.ron` PREVIEW (C2.1) — a
/// [`ScrollArea`](egui::ScrollArea) of [`ui.monospace`](egui::Ui::monospace) text, re-serialized
/// from the draft each frame so it tracks every edit. Hosted in the LEFT secondary strip since
/// GTW-534 C2 (demoted off the central region — kept + live, no longer dominating).
///
/// Projects the draft with its minted key if present, else the [`TerrainUuid::nil`] sentinel as a
/// placeholder (the real key is minted on save) — exactly the old `bevy_ui` preview's behavior.
/// Reuses [`draft_to_terrain_def`] + [`serialize_terrain_def`] verbatim (C2.2); a projection error
/// (the GTW-574 fail-closed Emplacement-without-weapon state) or a serialize error renders as an
/// inline marker rather than a panic — so the preview itself TELLS the author why the draft
/// cannot save yet.
pub(crate) fn ron_preview(ui: &mut egui::Ui, draft: &TerrainDraft) {
    ui.heading("Preview (.terrain_def.ron)");
    ui.separator();
    let key = draft.uuid().unwrap_or_else(TerrainUuid::nil);
    let body = match draft_to_terrain_def(draft, key) {
        Ok(def) => {
            serialize_terrain_def(&def).unwrap_or_else(|err| format!("<serialize error: {err}>"))
        }
        Err(err) => format!("<cannot project yet: {err}>"),
    };
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.monospace(body);
    });
}
