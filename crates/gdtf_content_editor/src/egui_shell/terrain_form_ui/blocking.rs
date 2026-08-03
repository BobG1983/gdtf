//! authoring knobs that override a def's kind-derived blocking: the path-blocking tri-state
use bevy_egui::egui;
use gdtf_battle_sim::terrain::def::LosBlocking;

use crate::terrain_form::TerrainDraft;

const LOS_ORDER: [LosBlocking; 3] = [
    LosBlocking::Full,
    LosBlocking::UpToHeightBand,
    LosBlocking::None,
];

const fn los_label(mode: LosBlocking) -> &'static str {
    match mode {
        LosBlocking::Full => "Full (whole storey)",
        LosBlocking::UpToHeightBand => "Up to height band",
        LosBlocking::None => "None (transparent)",
    }
}

pub(super) fn blocking_overrides(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    path_blocking_override(ui, draft);
    los_blocking_override(ui, draft);
}

fn path_blocking_override(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    ui.label("Path blocking override");
    let mut overridden = draft.blocks_pathing().is_some();
    if ui
        .checkbox(&mut overridden, "Override kind default")
        .changed()
    {
        draft.set_blocks_pathing(overridden.then_some(true));
    }
    if let Some(mut blocks) = draft.blocks_pathing()
        && ui.checkbox(&mut blocks, "Blocks pathing").changed()
    {
        draft.set_blocks_pathing(Some(blocks));
    }
}

fn los_blocking_override(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    ui.label("LoS blocking override");
    let mut overridden = draft.blocks_los().is_some();
    if ui
        .checkbox(&mut overridden, "Override kind default")
        .changed()
    {
        draft.set_blocks_los(overridden.then_some(LosBlocking::Full));
    }
    if let Some(mut mode) = draft.blocks_los() {
        let before = mode;
        egui::ComboBox::from_id_salt("terrain_los_blocking_combo")
            .selected_text(los_label(mode))
            .show_ui(ui, |ui| {
                for option in LOS_ORDER {
                    ui.selectable_value(&mut mode, option, los_label(option));
                }
            });
        if mode != before {
            draft.set_blocks_los(Some(mode));
        }
    }
}
