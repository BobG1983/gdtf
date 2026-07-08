//! The WEAPON form's FIRE-MODE list editor (GTW-670 C2) — the rows drawing the SHARED
//! [`fire_mode_edit`](crate::egui_shell::fire_mode_edit) widget. The slot-declaration
//! and attachment-key lists moved to the shared
//! [`slots_edit`](crate::egui_shell::slots_edit) sibling the melee form consumes too
//! (GTW-671 C2 — the contract's "same widget as ranged" clause).
//!
//! The [`FireMode`] sim newtype is construct-only (a private inner with a read
//! accessor), so the list edits by PROJECTING the authored list out, mutating it, and
//! folding it back through the SAME constructor the loader's deserialize uses — the
//! model stays the loader schema end to end. One structural edit (add / remove) folds
//! in per frame (the gang member-list precedent; a click is a discrete event, so the
//! fold is idempotent under the egui multipass re-run).

use bevy_egui::egui;
use gdtf_battle_sim::weapon::{FireMode, FireModeSpec, WeaponSpec};

use crate::{egui_shell::fire_mode_edit, weapon_form::structural_single_mode};

/// The FIRE-MODE list — one row per authored [`FireModeSpec`] via the SHARED
/// [`fire_mode_edit::fire_mode_row`] widget (GTW-670 C2 — the same authoring surface
/// the ATTACHMENT mode's `GainFireMode` rows draw, never a fork) plus Remove, and the
/// Add-mode button seeding the structural single-shot template. Remove is DISABLED at
/// one mode: the [`FireMode`] invariant documents "a well-authored weapon lists at
/// least one mode", so a mode-less weapon is unauthorable by construction (the
/// injuries `effects >= 1` floor precedent).
pub(super) fn fire_modes_list(ui: &mut egui::Ui, spec: &mut WeaponSpec) {
    let mut modes: Vec<FireModeSpec> = spec.fire_mode.to_vec();
    let single_mode = modes.len() == 1;
    let mut remove: Option<usize> = None;
    for (index, mode) in modes.iter_mut().enumerate() {
        ui.group(|ui| {
            fire_mode_edit::fire_mode_row(ui, index, mode);
            if ui
                .add_enabled(!single_mode, egui::Button::new("Remove mode"))
                .clicked()
            {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        modes.remove(index);
    }
    if ui.button("Add fire mode").clicked() {
        modes.push(structural_single_mode());
    }
    // Fold the edited list back through the loader's own constructor.
    spec.fire_mode = FireMode::new(modes);
}
