//! On-death list editor for a terrain draft — same rows as the weapon form.
use bevy_egui::egui;
use gdtf_battle_sim::effects::{
    fields::FieldKey,
    on_death::{ExplodeDamage, OnDeathEffect},
};

use crate::{
    egui_shell::{damage_edit::damage_type_combo, fire_mode_edit},
    terrain_form::TerrainDraft,
    weapon_form::{explode_template, leave_field_template},
};

// What a row's own buttons asked for, applied once the loop has let the list go.
enum RowButton {
    Remove(usize),
    Up(usize),
    Down(usize),
}

pub(super) fn on_death_form(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    ui.horizontal(|ui| {
        ui.label("on death effects");
        if ui.button("add").clicked() {
            draft.add_on_death(explode_template());
        }
    });
    let mut pressed: Option<RowButton> = None;
    for index in 0..draft.on_death().len() {
        let Some(stored) = draft.on_death().get(index).cloned() else {
            continue;
        };
        let mut edited = stored.clone();
        ui.push_id(index, |ui| {
            row_buttons(ui, index, &mut pressed);
            variant_combo(ui, &mut edited);
            payload_rows(ui, &mut edited);
        });
        if edited != stored {
            draft.set_on_death_at(index, edited);
        }
    }
    match pressed {
        Some(RowButton::Remove(index)) => {
            draft.remove_on_death(index);
        }
        Some(RowButton::Up(index)) => {
            draft.move_on_death_up(index);
        }
        Some(RowButton::Down(index)) => {
            draft.move_on_death_down(index);
        }
        None => {}
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
            fire_mode_edit::hit_type_row(ui, "terrain_on_death_hit_type", hit_type);
            ui.horizontal(|ui| {
                ui.label("blast damage");
                let mut drain = **damage;
                if ui.add(egui::DragValue::new(&mut drain)).changed() {
                    *damage = ExplodeDamage::new(drain);
                }
            });
            damage_type_combo(ui, "terrain_on_death_damage_type", damage_type);
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
        egui::ComboBox::from_id_salt("terrain_on_death_variant")
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
