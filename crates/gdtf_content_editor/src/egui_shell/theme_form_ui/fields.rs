//! The THEME tab's RIGHT mode-form FIELD STACK (GTW-530 C2/C3) — display name, the SLAB-only
//! default-floor combo, the read-only UUID, the New-theme button, and the debug-only Save press.

use bevy_egui::egui;
use gdtf_battle_sim::terrain::def::TerrainDefRegistry;

use crate::theme_form::{ThemeDraft, slab_floor_candidates};

/// The placeholder UUID line shown for a not-yet-meaningful theme key (e.g. before any load).
const KEY_PREFIX: &str = "UUID: ";

/// Draw the THEME-mode FIELD STACK into the RIGHT mode-form panel (GTW-530 C2/C3) — display name,
/// the default-floor [`ComboBox`] (SLAB-ONLY via [`slab_floor_candidates`] — GTW-530 C3), the
/// read-only UUID key, the New-theme button (mints a fresh draft), and the debug-only Save button.
///
/// The terrain library multi-select is NO LONGER drawn here (GTW-530 C2): it is the CENTRAL primary
/// region ([`terrain_library_panel`](super::terrain_library_panel)) so the largest space showcases the terrain + its sprites.
///
/// Every control reads / writes the [`ThemeDraft`] through its existing accessors / setters
/// (C3.3): the floor combo routes through [`ThemeDraft::set_default_floor`] (which ignores a key
/// not in the palette — fail-closed C6).
pub(crate) fn field_stack(
    ui: &mut egui::Ui,
    draft: &mut ThemeDraft,
    terrain: Option<&TerrainDefRegistry>,
) {
    ui.heading("Theme");
    ui.separator();

    name_field(ui, draft);
    floor_combo(ui, draft, terrain);
    uuid_text(ui, draft);

    ui.separator();
    new_theme_button(ui, draft);

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_button(ui, draft);
    }
}

/// The display-name text field — a single-line edit committed straight into the draft (C3.1).
fn name_field(ui: &mut egui::Ui, draft: &mut ThemeDraft) {
    ui.label("Display name");
    let mut name = draft.display_name().to_owned();
    if ui.text_edit_singleline(&mut name).changed() {
        draft.set_display_name(name);
    }
}

/// The default-floor [`ComboBox`] (GTW-530 C3) — offers ONLY the draft's own SLAB-kind terrain as
/// candidates (via [`slab_floor_candidates`]), with the current default floor pre-selected. The
/// walkable floor is always a slab, so no Wall / Cover terrain ever appears in this picker.
/// Choosing a row routes through [`ThemeDraft::set_default_floor`], which ignores it unless the
/// terrain is in the palette (fail-closed C6). When no slab is selected the combo shows `"(none)"`
/// and is disabled. Requires the terrain registry to resolve display names; falls back to a
/// read-only label when absent.
fn floor_combo(ui: &mut egui::Ui, draft: &mut ThemeDraft, terrain: Option<&TerrainDefRegistry>) {
    ui.label("Default floor");
    let Some(reg) = terrain else {
        ui.label("(loading…)");
        return;
    };
    let candidates = slab_floor_candidates(draft, reg);
    if candidates.is_empty() {
        ui.add_enabled_ui(false, |ui| {
            egui::ComboBox::from_id_salt("theme_floor_combo")
                .selected_text("(none)")
                .show_ui(ui, |_ui| {});
        });
        return;
    }
    let current_floor = draft.default_floor();
    let preview = current_floor
        .and_then(|key| candidates.iter().find(|(k, _)| *k == key))
        .map_or_else(|| "(select…)".to_owned(), |(_, label)| label.clone());
    let mut chosen = current_floor;
    let mut clicked = false;
    egui::ComboBox::from_id_salt("theme_floor_combo")
        .selected_text(preview)
        .show_ui(ui, |ui| {
            for (key, label) in &candidates {
                let is_selected = Some(*key) == current_floor;
                if ui.selectable_label(is_selected, label).clicked() {
                    chosen = Some(*key);
                    clicked = true;
                }
            }
        });
    if clicked && let Some(key) = chosen {
        draft.set_default_floor(key);
    }
}

/// The read-only UUID line (C3.1) — the draft's theme key shown as a formatted UUID.
fn uuid_text(ui: &mut egui::Ui, draft: &ThemeDraft) {
    let key_str = format!("{}{}", KEY_PREFIX, *draft.key());
    ui.label(key_str);
}

/// The New-theme button (C3.1) — on press, replaces the draft with a freshly minted
/// [`ThemeDraft::new_theme`] (a new UUID, empty name, no terrain, no floor). Idempotent under
/// the egui multipass re-run (the second press is a distinct click event, not a duplicate).
fn new_theme_button(ui: &mut egui::Ui, draft: &mut ThemeDraft) {
    if ui.button("New theme").clicked() {
        *draft = ThemeDraft::new_theme();
    }
}

/// The debug-only Save button (C3.1) — on press it validates the draft (
/// [`validate_for_save`](crate::theme_form::validate_for_save)) and calls the existing
/// [`write_theme`](crate::theme_form::write_theme) (C3.3). On a typed error it logs and writes
/// nothing (never a panic). Debug-only — the whole theme save path is gated
/// `#[cfg(debug_assertions)]`.
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &ThemeDraft) {
    if !ui.button("Save theme").clicked() {
        return;
    }
    let key = draft.key();
    match crate::theme_form::write_theme(draft, key) {
        Ok(path) => bevy::log::info!("theme save: wrote theme def to `{}`", path.display()),
        Err(err) => bevy::log::error!("theme save: {err}"),
    }
}
