//! Fire-mode list widget for the WEAPON tab. The draft owns the min-one rule and every write.
use bevy_egui::egui;
use gdtf_battle_sim::weapon::FireModeSpec;

use crate::{egui_shell::fire_mode_edit, weapon_form::WeaponDraft};

pub(super) fn fire_modes_list(ui: &mut egui::Ui, draft: &mut WeaponDraft) {
    let mut modes: Vec<FireModeSpec> = draft.fire_modes().to_vec();
    let removable = draft.can_remove_fire_mode();
    let mut remove: Option<usize> = None;
    for (index, mode) in modes.iter_mut().enumerate() {
        ui.group(|ui| {
            fire_mode_edit::fire_mode_row(ui, index, mode);
            if ui
                .add_enabled(removable, egui::Button::new("Remove mode"))
                .clicked()
            {
                remove = Some(index);
            }
        });
    }
    for (index, mode) in modes.iter().enumerate() {
        draft.set_fire_mode(index, *mode);
    }
    if let Some(index) = remove {
        draft.remove_fire_mode(index);
    }
    if ui.button("Add fire mode").clicked() {
        draft.add_fire_mode();
    }
}
