//! Optional weapon fields; unchecked means authored `None`.
use bevy_egui::egui;
use gdtf_battle_sim::{
    effects::{
        fields::FieldKey,
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    weapon::{DotDamage, DotProfile, WeaponSpec},
};

use crate::{
    egui_shell::{damage_edit::damage_type_combo, fire_mode_edit},
    weapon_form::{dot_turns_from_raw, explode_template, leave_field_template},
};

pub(super) fn dot_form(ui: &mut egui::Ui, spec: &mut WeaponSpec) {
    let mut enabled = spec.dot.is_some();
    if ui
        .checkbox(&mut enabled, "damage over time (dot)")
        .changed()
    {
        spec.dot = enabled.then(DotProfile::default);
    }
    let Some(dot) = spec.dot.as_mut() else {
        return;
    };
    ui.horizontal(|ui| {
        ui.label("per-turn damage");
        let mut damage = *dot.damage;
        if ui.add(egui::DragValue::new(&mut damage)).changed() {
            dot.damage = DotDamage::new(damage);
        }
        ui.label("turns");
        let mut turns = dot.turns.get();
        if ui
            .add(egui::DragValue::new(&mut turns).range(1..=u8::MAX))
            .changed()
        {
            dot.turns = dot_turns_from_raw(turns);
        }
    });
    damage_type_combo(ui, "weapon_dot_damage_type", &mut dot.damage_type);
}

pub(super) fn on_death_form(ui: &mut egui::Ui, spec: &mut WeaponSpec) {
    let mut enabled = spec.on_death.is_some();
    if ui.checkbox(&mut enabled, "on death effect").changed() {
        spec.on_death = enabled.then(explode_template);
    }
    let Some(effect) = spec.on_death.as_mut() else {
        return;
    };
    variant_combo(ui, effect);
    match effect {
        OnDeathEffect::Explode {
            hit_type,
            damage,
            damage_type,
        } => {
            fire_mode_edit::hit_type_row(ui, "weapon_on_death_hit_type", hit_type);
            ui.horizontal(|ui| {
                ui.label("blast damage");
                let mut drain = **damage;
                if ui.add(egui::DragValue::new(&mut drain)).changed() {
                    *damage = ExplodeDamage::new(drain);
                }
            });
            damage_type_combo(ui, "weapon_on_death_damage_type", damage_type);
        }
        OnDeathEffect::LeaveField { field } => {
            ui.horizontal(|ui| {
                ui.label("field key");
                let mut key = field.as_str().to_owned();
                if ui.text_edit_singleline(&mut key).changed() {
                    *field = FieldKey::new(key);
                }
            });
        }
    }
}

const fn effect_label(effect: &OnDeathEffect) -> &'static str {
    match effect {
        OnDeathEffect::Explode { .. } => "Explode",
        OnDeathEffect::LeaveField { .. } => "LeaveField",
    }
}

fn variant_combo(ui: &mut egui::Ui, effect: &mut OnDeathEffect) {
    let current = effect_label(effect);
    ui.horizontal(|ui| {
        ui.label("effect");
        egui::ComboBox::from_id_salt("weapon_on_death_variant")
            .selected_text(current)
            .show_ui(ui, |ui| {
                for template in [explode_template(), leave_field_template()] {
                    let label = effect_label(&template);
                    if ui.selectable_label(current == label, label).clicked() && current != label {
                        *effect = template;
                    }
                }
            });
    });
}
