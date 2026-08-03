//! both consume THIS widget, so the shared six fields have ONE authoring surface —
use bevy_egui::egui;
use gdtf_battle_sim::weapon::{
    DamageType, FatalBias, Handedness, WeaponDamage, WeaponPunch, WeaponShred,
};

const HANDEDNESS_OPTIONS: [Handedness; 2] = [Handedness::OneHanded, Handedness::TwoHanded];

pub(in crate::egui_shell) struct DamageGroupFields<'a> {
        pub(in crate::egui_shell) damage:      &'a mut WeaponDamage,
        pub(in crate::egui_shell) punch:       &'a mut WeaponPunch,
        pub(in crate::egui_shell) shred:       &'a mut WeaponShred,
        pub(in crate::egui_shell) damage_type: &'a mut DamageType,
        pub(in crate::egui_shell) fatal_bias:  &'a mut FatalBias,
        pub(in crate::egui_shell) handedness:  &'a mut Handedness,
}

pub(in crate::egui_shell) fn damage_group(
    ui: &mut egui::Ui,
    salt: &'static str,
    fields: DamageGroupFields<'_>,
) {
    drag_i32(ui, "damage", **fields.damage, |v| {
        *fields.damage = WeaponDamage::new(v);
    });
    drag_i32(ui, "punch", **fields.punch, |v| {
        *fields.punch = WeaponPunch::new(v);
    });
    drag_i32(ui, "shred", **fields.shred, |v| {
        *fields.shred = WeaponShred::new(v);
    });
    damage_type_combo(ui, (salt, "damage_type"), fields.damage_type);
    drag_f32(ui, "fatal_bias", 0.05, **fields.fatal_bias, |v| {
        *fields.fatal_bias = FatalBias::new(v);
    });
    ui.horizontal(|ui| {
        ui.label("handedness");
        egui::ComboBox::from_id_salt((salt, "handedness"))
            .selected_text(format!("{:?}", fields.handedness))
            .show_ui(ui, |ui| {
                for option in HANDEDNESS_OPTIONS {
                    ui.selectable_value(fields.handedness, option, format!("{option:?}"));
                }
            });
    });
}

pub(in crate::egui_shell) fn damage_type_combo(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    ty: &mut DamageType,
) {
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

pub(in crate::egui_shell) fn drag_f32(
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

pub(in crate::egui_shell) fn drag_i32(
    ui: &mut egui::Ui,
    label: &str,
    value: i32,
    commit: impl FnOnce(i32),
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut edited = value;
        if ui.add(egui::DragValue::new(&mut edited)).changed() {
            commit(edited);
        }
    });
}
