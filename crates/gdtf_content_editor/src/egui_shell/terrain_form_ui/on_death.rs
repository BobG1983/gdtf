//! On-death editor for a terrain draft — same variants as the weapon form.
use bevy_egui::egui;
use gdtf_battle_sim::{
    effects::{
        fields::FieldKey,
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    weapon::{DamageType, HitType},
};

use crate::{
    egui_shell::{damage_edit::damage_type_combo, fire_mode_edit},
    terrain_form::TerrainDraft,
};

pub(super) fn on_death_form(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    let mut enabled = draft.on_death().is_some();
    if ui.checkbox(&mut enabled, "on death effect").changed() {
        draft.set_on_death(enabled.then(explode_template));
    }
    let Some(effect) = draft.on_death_mut() else {
        return;
    };
    variant_combo(ui, effect);
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

const fn explode_template() -> OnDeathEffect {
    OnDeathEffect::Explode {
        hit_type:    HitType::Single,
        damage:      ExplodeDamage::new(0),
        damage_type: DamageType::Kinetic,
    }
}

const fn leave_field_template() -> OnDeathEffect {
    OnDeathEffect::LeaveField {
        field: FieldKey::new(String::new()),
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
