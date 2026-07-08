//! The WEAPON form's STATS / HANDLING / MAGAZINE groups (GTW-670 C2) — the three
//! ranged-only ballistics scalar drags, the trajectory combo plus the two boolean tags,
//! and the authored magazine pair. The six SHARED damage-group fields (`damage` /
//! `punch` / `shred` / `damage_type` / `fatal_bias` / `handedness`) moved to the shared
//! [`damage_edit`](crate::egui_shell::damage_edit) widget the melee form consumes too
//! (GTW-671 C2 — one source, never a fork); the generic scalar drags ride with it.
//!
//! NO drag clamp is invented (the GTW-479 ruling, restated by the shared
//! `fire_mode_edit` widget): every weapon scalar's magnitude is documented tuning-open
//! ("TBD tuning" — `equipment/weapon/components/*.rs` + `docs/combat/weapons-and-armor.md`),
//! so the drags span their payload TYPES' own ranges; the unsigned counts are
//! non-negative by construction.

use bevy_egui::egui;
use gdtf_battle_sim::{
    magazine::ReloadTu,
    weapon::{
        Accuracy, BaseSpread, Kickback, MagazineSize, Shove, Stable, TrajectoryStyle, WeaponSpec,
    },
};

use crate::egui_shell::damage_edit::drag_f32;

/// The two closed [`TrajectoryStyle`] variants the combo offers, in the sim's
/// declaration order (GTW-546 — flat ray vs lobbed arc).
const TRAJECTORY_OPTIONS: [TrajectoryStyle; 2] = [TrajectoryStyle::Straight, TrajectoryStyle::Arc];

/// The STATS group — the three RANGED-ONLY ballistics scalar drags (`base_spread` /
/// `accuracy` / `kickback`), each folding through its sim newtype's constructor
/// (GTW-670 C2). The floats carry NO documented bounds (tuning-open — the GTW-479
/// ruling), so no clamp is invented. The four shared damage scalars draw in the
/// [`damage_edit`](crate::egui_shell::damage_edit) group since GTW-671.
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

/// The HANDLING group — the RANGED-ONLY trajectory combo plus the two boolean weapon
/// tags (`stable` / `shove`), each committed straight into the sim record (GTW-670 C2).
/// The `damage_type` / `handedness` combos draw in the shared
/// [`damage_edit`](crate::egui_shell::damage_edit) group since GTW-671.
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

/// The MAGAZINE group — the two AUTHORED fields only (`size` + `reload_tu`; the live
/// `rounds` count is spawn-only and never authored — GTW-670 C2), edited through the
/// sim [`Magazine`](gdtf_battle_sim::weapon::Magazine) grouping's own fields. Unsigned
/// counts span their types' own ranges (no documented tighter bound).
pub(super) fn magazine_group(ui: &mut egui::Ui, spec: &mut WeaponSpec) {
    ui.horizontal(|ui| {
        ui.label("size");
        let mut size = *spec.magazine.size;
        if ui.add(egui::DragValue::new(&mut size)).changed() {
            spec.magazine.size = MagazineSize::new(size);
        }
        ui.label("reload_tu");
        let mut reload = *spec.magazine.reload_tu;
        if ui.add(egui::DragValue::new(&mut reload)).changed() {
            spec.magazine.reload_tu = ReloadTu::new(reload);
        }
    });
}
