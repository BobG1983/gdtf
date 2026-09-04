//! Weapon form damage and magazine fields.
use bevy_egui::egui;
use gdtf_battle_sim::{
    magazine::ReloadTu,
    weapon::{
        Accuracy, BaseSpread, Kickback, MagazineSize, Shove, Stable, TrajectoryStyle, WeaponSpec,
    },
};

use crate::egui_shell::damage_edit::drag_f32;

const TRAJECTORY_OPTIONS: [TrajectoryStyle; 2] = [TrajectoryStyle::Straight, TrajectoryStyle::Arc];

pub(super) fn stats_group(ui: &mut egui::Ui, spec: &mut WeaponSpec) {
    drag_f32(ui, "base_spread", 0.01, *spec.base_spread, |v| {
        spec.base_spread = BaseSpread::new(v);
    });
    drag_f32(ui, "accuracy", 0.05, *spec.accuracy, |v| {
        spec.accuracy = Accuracy::new(v);
    });
    drag_f32(ui, "kickback", 0.01, *spec.kickback, |v| {
        spec.kickback = Kickback::new(v);
    });
}

pub(super) fn handling_group(ui: &mut egui::Ui, spec: &mut WeaponSpec) {
    ui.horizontal(|ui| {
        ui.label("trajectory");
        egui::ComboBox::from_id_salt("weapon_trajectory")
            .selected_text(format!("{:?}", spec.trajectory))
            .show_ui(ui, |ui| {
                for option in TRAJECTORY_OPTIONS {
                    ui.selectable_value(&mut spec.trajectory, option, format!("{option:?}"));
                }
            });
    });
    let mut stable = *spec.stable;
    if ui
        .checkbox(&mut stable, "stable (braced-by-design)")
        .changed()
    {
        spec.stable = Stable::new(stable);
    }
    let mut shove = *spec.shove;
    if ui
        .checkbox(&mut shove, "shove (knock-back on connect)")
        .changed()
    {
        spec.shove = Shove::new(shove);
    }
}

pub(super) fn magazine_group(ui: &mut egui::Ui, spec: &mut WeaponSpec) {
    ui.horizontal(|ui| {
        ui.label("size");
        let mut size = *spec.magazine.size();
        if ui.add(egui::DragValue::new(&mut size)).changed() {
            spec.magazine.set_size(MagazineSize::new(size));
        }
        ui.label("reload_tu");
        let mut reload = *spec.magazine.reload_tu();
        if ui.add(egui::DragValue::new(&mut reload)).changed() {
            spec.magazine.set_reload_tu(ReloadTu::new(reload));
        }
    });
}
