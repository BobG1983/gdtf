//! The WEAPON form's STATS / HANDLING / MAGAZINE groups (GTW-670 C2) — the seven
//! ballistics/damage scalar drags, the three closed-vocabulary combos plus the two
//! boolean tags, and the authored magazine pair.
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
        Accuracy, BaseSpread, DamageType, FatalBias, Handedness, Kickback, MagazineSize, Shove,
        Stable, TrajectoryStyle, WeaponDamage, WeaponPunch, WeaponShred, WeaponSpec,
    },
};

/// The two closed [`Handedness`] variants the combo offers, in the sim's declaration
/// order (GTW-443 — the hand-count vocabulary).
const HANDEDNESS_OPTIONS: [Handedness; 2] = [Handedness::OneHanded, Handedness::TwoHanded];

/// The two closed [`TrajectoryStyle`] variants the combo offers, in the sim's
/// declaration order (GTW-546 — flat ray vs lobbed arc).
const TRAJECTORY_OPTIONS: [TrajectoryStyle; 2] = [TrajectoryStyle::Straight, TrajectoryStyle::Arc];

/// The STATS group — the seven scalar drags (`base_spread` / `accuracy` / `kickback` /
/// `fatal_bias` / `damage` / `punch` / `shred`), each folding through its sim newtype's
/// constructor (GTW-670 C2). The floats carry NO documented bounds (tuning-open — the
/// GTW-479 ruling), so no clamp is invented; the signed damage triple spans the honest
/// `i32` the per-hit formula subtracts with.
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
    drag_f32(ui, "fatal_bias", 0.05, *spec.fatal_bias, |v| {
        spec.fatal_bias = FatalBias::new(v);
    });
    drag_i32(ui, "damage", *spec.damage, |v| {
        spec.damage = WeaponDamage::new(v);
    });
    drag_i32(ui, "punch", *spec.punch, |v| {
        spec.punch = WeaponPunch::new(v);
    });
    drag_i32(ui, "shred", *spec.shred, |v| {
        spec.shred = WeaponShred::new(v);
    });
}

/// The HANDLING group — the three closed-vocabulary combos (`damage_type` /
/// `handedness` / `trajectory`) plus the two boolean weapon tags (`stable` / `shove`),
/// each committed straight into the sim record (GTW-670 C2).
pub(super) fn handling_group(ui: &mut egui::Ui, spec: &mut WeaponSpec) {
    damage_type_combo(ui, "weapon_damage_type", &mut spec.damage_type);
    ui.horizontal(|ui| {
        ui.label("handedness");
        egui::ComboBox::from_id_salt("weapon_handedness")
            .selected_text(format!("{:?}", spec.handedness))
            .show_ui(ui, |ui| {
                for option in HANDEDNESS_OPTIONS {
                    ui.selectable_value(&mut spec.handedness, option, format!("{option:?}"));
                }
            });
    });
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

/// A labelled [`DamageType`] combo over the seven wheel nodes in the sim's canonical
/// [`DamageType::ALL`] order (the attachment form's convention: debug-formatted labels
/// ARE the authored RON identifiers). Shared by the HANDLING group and the DOT sub-form
/// (distinct salts).
pub(super) fn damage_type_combo(ui: &mut egui::Ui, salt: &'static str, ty: &mut DamageType) {
    ui.horizontal(|ui| {
        ui.label("damage_type");
        egui::ComboBox::from_id_salt(salt)
            .selected_text(format!("{ty:?}"))
            .show_ui(ui, |ui| {
                for option in DamageType::ALL {
                    ui.selectable_value(ty, option, format!("{option:?}"));
                }
            });
    });
}

/// A labelled `f32` stat drag on its own row, committing through `commit` only on a
/// change (the attachment payload-drag shape — the stats group has four float fields).
pub(super) fn drag_f32(
    ui: &mut egui::Ui,
    label: &str,
    speed: f64,
    value: f32,
    commit: impl FnOnce(f32),
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut edited = value;
        if ui
            .add(egui::DragValue::new(&mut edited).speed(speed))
            .changed()
        {
            commit(edited);
        }
    });
}

/// A labelled `i32` stat drag — the signed-magnitude twin of [`drag_f32`] (damage /
/// punch / shred share the sim's honest-signed `i32`).
pub(super) fn drag_i32(ui: &mut egui::Ui, label: &str, value: i32, commit: impl FnOnce(i32)) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut edited = value;
        if ui.add(egui::DragValue::new(&mut edited)).changed() {
            commit(edited);
        }
    });
}
