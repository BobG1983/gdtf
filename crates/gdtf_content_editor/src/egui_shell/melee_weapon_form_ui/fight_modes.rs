//! no dispersion cone), so reusing the ranged row would author fields the melee spec
use bevy_egui::egui;
use gdtf_battle_sim::weapon::{
    FightMode, FightModeKind, FightModeSpec, MeleeWeaponSpec, Strikes, TuCost,
};

use crate::melee_weapon_form::structural_swing_mode;

const FIGHT_MODE_KINDS: [FightModeKind; 2] = [FightModeKind::Swing, FightModeKind::Thrust];

pub(super) fn fight_modes_list(ui: &mut egui::Ui, spec: &mut MeleeWeaponSpec) {
    let mut modes: Vec<FightModeSpec> = spec.fight_mode.to_vec();
    let single_mode = modes.len() == 1;
    let mut remove: Option<usize> = None;
    for (index, mode) in modes.iter_mut().enumerate() {
        ui.group(|ui| {
            fight_mode_row(ui, index, mode);
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
    if ui.button("Add fight mode").clicked() {
        modes.push(structural_swing_mode());
    }
    spec.fight_mode = FightMode::new(modes);
}

fn fight_mode_row(ui: &mut egui::Ui, index: usize, spec: &mut FightModeSpec) {
    ui.horizontal(|ui| {
        ui.label("Kind");
        egui::ComboBox::from_id_salt(("fight_mode_kind", index))
            .selected_text(format!("{:?}", spec.kind))
            .show_ui(ui, |ui| {
                for option in FIGHT_MODE_KINDS {
                    ui.selectable_value(&mut spec.kind, option, format!("{option:?}"));
                }
            });
        ui.label("TU");
        let mut tu = *spec.tu_cost;
        if ui.add(egui::DragValue::new(&mut tu)).changed() {
            spec.tu_cost = TuCost::new(tu);
        }
        ui.label("strikes");
        let mut strikes = *spec.strikes;
        if ui.add(egui::DragValue::new(&mut strikes)).changed() {
            spec.strikes = Strikes::new(strikes);
        }
    });
}
