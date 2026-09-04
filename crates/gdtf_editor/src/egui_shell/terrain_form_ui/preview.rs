use bevy_egui::egui;
use gdtf_battle_sim::terrain::def::TerrainUuid;

use crate::terrain_form::{TerrainDraft, draft_to_terrain_def, serialize_terrain_def};

/// Draw the live monospace `.terrain_def.ron` PREVIEW (C2.1) — a
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
