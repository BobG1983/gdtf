//! Optional weapon fields: the DOT tick box and the on-death list.
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

// What a row's own buttons asked for, applied once the loop has let the list go.
enum RowButton {
    Remove(usize),
    Up(usize),
    Down(usize),
}

pub(super) fn on_death_form(ui: &mut egui::Ui, spec: &mut WeaponSpec) {
    ui.horizontal(|ui| {
        ui.label("on death effects");
        if ui.button("add").clicked() {
            spec.on_death.push(explode_template());
        }
    });
    let mut pressed: Option<RowButton> = None;
    for (index, effect) in spec.on_death.iter_mut().enumerate() {
        ui.push_id(index, |ui| {
            row_buttons(ui, index, &mut pressed);
            variant_combo(ui, effect);
            payload_rows(ui, effect);
        });
    }
    match pressed {
        Some(RowButton::Remove(index)) if index < spec.on_death.len() => {
            spec.on_death.remove(index);
        }
        Some(RowButton::Up(index)) if index > 0 && index < spec.on_death.len() => {
            spec.on_death.swap(index - 1, index);
        }
        Some(RowButton::Down(index)) if index + 1 < spec.on_death.len() => {
            spec.on_death.swap(index, index + 1);
        }
        Some(_) | None => {}
    }
}

fn row_buttons(ui: &mut egui::Ui, index: usize, pressed: &mut Option<RowButton>) {
    ui.horizontal(|ui| {
        if ui.button("remove").clicked() {
            *pressed = Some(RowButton::Remove(index));
        }
        if ui.button("move up").clicked() {
            *pressed = Some(RowButton::Up(index));
        }
        if ui.button("move down").clicked() {
            *pressed = Some(RowButton::Down(index));
        }
    });
}

fn payload_rows(ui: &mut egui::Ui, effect: &mut OnDeathEffect) {
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
