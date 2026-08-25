//! Fight-mode list widget for the MELEE WEAPON tab. The draft owns the min-one rule and every
//! write.
use bevy_egui::egui;
use gdtf_battle_sim::weapon::{FightModeKind, FightModeSpec, Strikes, TuCost};

use crate::melee_weapon_form::MeleeWeaponDraft;

const FIGHT_MODE_KINDS: [FightModeKind; 2] = [FightModeKind::Swing, FightModeKind::Thrust];

pub(super) fn fight_modes_list(ui: &mut egui::Ui, draft: &mut MeleeWeaponDraft) {
    let mut modes: Vec<FightModeSpec> = draft.fight_modes().to_vec();
    let removable = draft.can_remove_fight_mode();
    let mut remove: Option<usize> = None;
    for (index, mode) in modes.iter_mut().enumerate() {
        ui.group(|ui| {
            fight_mode_row(ui, index, mode);
            if ui
                .add_enabled(removable, egui::Button::new("Remove mode"))
                .clicked()
            {
                remove = Some(index);
            }
        });
    }
    for (index, mode) in modes.iter().enumerate() {
        draft.set_fight_mode(index, *mode);
    }
    if let Some(index) = remove {
        draft.remove_fight_mode(index);
    }
    if ui.button("Add fight mode").clicked() {
        draft.add_fight_mode();
    }
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
