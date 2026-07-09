//! The TERRAIN tab's **per-def blocking-override controls** (GTW-587) — the two OPTIONAL
//! authoring knobs that override a def's kind-derived blocking: the path-blocking tri-state
//! and the height-banded line-of-sight blocking mode.
//!
//! Split out of [`fields`](super::fields) (which was already in the module-layout WARN band)
//! so the already-large field stack does not grow further. Each control reads / writes the
//! [`TerrainDraft`] through its GTW-587 accessors ([`blocks_pathing`] /
//! [`blocks_los`] + setters), matching the form's existing accessor-driven idiom — an
//! "Override kind default" checkbox toggles the `Option` on/off, and the value control appears
//! only while the override is engaged.
//!
//! [`blocks_pathing`]: TerrainDraft::blocks_pathing
//! [`blocks_los`]: TerrainDraft::blocks_los

use bevy_egui::egui;
use gdtf_battle_sim::terrain::def::LosBlocking;

use crate::terrain_form::TerrainDraft;

/// The [`LosBlocking`] mode order the LoS-override combo renders, top to bottom.
const LOS_ORDER: [LosBlocking; 3] = [
    LosBlocking::Full,
    LosBlocking::UpToHeightBand,
    LosBlocking::None,
];

/// The display label for a [`LosBlocking`] mode in the override combo.
const fn los_label(mode: LosBlocking) -> &'static str {
    match mode {
        LosBlocking::Full => "Full (whole storey)",
        LosBlocking::UpToHeightBand => "Up to height band",
        LosBlocking::None => "None (transparent)",
    }
}

/// Draw the two GTW-587 blocking-override controls (path-blocking + LoS-blocking) over the
/// draft. Called from [`field_stack`](super::fields::field_stack) after the tag checkboxes.
pub(super) fn blocking_overrides(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    path_blocking_override(ui, draft);
    los_blocking_override(ui, draft);
}

/// The path-blocking override (GTW-587) — an "Override kind default" checkbox that engages the
/// tri-state; while engaged, a "Blocks pathing" checkbox sets the forced boolean. Disengaging
/// clears the override back to `None` (the kind default resolves at battle setup). Enabling it
/// starts at `Some(true)` (force-block), the common authoring intent (a railing / low wall).
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

/// The LoS-blocking override (GTW-587) — an "Override kind default" checkbox that engages the
/// override; while engaged, a [`ComboBox`](egui::ComboBox) picks the height-banded
/// [`LosBlocking`] mode. Disengaging clears it back to `None` (the kind default). Enabling it
/// starts at [`LosBlocking::Full`] (occlude the whole storey).
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
