//! accessor), so the list edits by PROJECTING the authored list out, mutating it, and
use bevy_egui::egui;
use gdtf_battle_sim::weapon::{FireMode, FireModeSpec, WeaponSpec};

use crate::{egui_shell::fire_mode_edit, weapon_form::structural_single_mode};

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
    spec.fire_mode = FireMode::new(modes);
}
