//! The WEAPON form's two OPTIONAL sub-forms (GTW-670 C2) — the `dot:`
//! [`DotProfile`] and the `on_death:` [`OnDeathEffect`], each behind an enable
//! checkbox (an unchecked box IS the authored `None` — the serde-default identity).
//!
//! Enable seeds carry DOCUMENTED identities the author immediately re-tunes (the
//! GTW-669 template rule): the DOT seed is the sim's minimal ONE-turn zero-damage
//! profile (a zero-TURN profile is unrepresentable — GTW-643; the form's drag clamps
//! to `1..=u8::MAX` and the [`dot_turns_from_raw`] fold backstops it), and the
//! on-death seed is a `Single`-template zero-damage `Explode`.

use bevy_egui::egui;
use gdtf_battle_sim::{
    effects::{
        fields::FieldKey,
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    weapon::{DotDamage, DotProfile, HitType, WeaponSpec},
};

use crate::{
    egui_shell::{damage_edit::damage_type_combo, fire_mode_edit},
    weapon_form::dot_turns_from_raw,
};

/// The DOT sub-form — the enable checkbox toggling the authored
/// `Option<DotProfile>`, then the per-turn damage drag (the `u16` type's own range),
/// the flavour [`damage_type_combo`], and the turns drag clamped to `1..=u8::MAX`
/// (GTW-670 C2: an authored `0` is unrepresentable in the form, matching the
/// [`DotTurns`](gdtf_battle_sim::weapon::DotTurns) type — GTW-643).
pub(super) fn dot_form(ui: &mut egui::Ui, spec: &mut WeaponSpec) {
    let mut enabled = spec.dot.is_some();
    if ui
        .checkbox(&mut enabled, "damage over time (dot)")
        .changed()
    {
        // The seed is the sim's documented minimal profile (one turn, zero damage) —
        // a starting point, never a design claim; unchecking authors the None
        // identity back.
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

/// The ON-DEATH sub-form — the enable checkbox toggling the authored
/// `Option<OnDeathEffect>`, the closed two-variant combo (`Explode` / `LeaveField`),
/// and that variant's payload fields (GTW-670 C2). Picking a DIFFERENT variant
/// replaces the value with that kind's template (the current kind's row is a no-op,
/// so an open combo never wipes a tuned payload — the injury variant-combo
/// convention).
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
            // The AoE template row is the SHARED fire_mode_edit hit-type widget
            // (extended in place for GTW-670, not forked).
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
            // The GTW-545 field catalog key — a free text reference (the editor
            // loads no FieldDefRegistry, so there is no option source to combo
            // over; the key resolves fail-closed at runtime).
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

/// The `Explode` enable/switch template — the `Single` degenerate template (just the
/// death cell), zero drain, the documented `Kinetic` default: every payload seeds an
/// identity or documented default (the GTW-669 template rule).
const fn explode_template() -> OnDeathEffect {
    OnDeathEffect::Explode {
        hit_type:    HitType::Single,
        damage:      ExplodeDamage::new(0),
        damage_type: gdtf_battle_sim::weapon::DamageType::Kinetic,
    }
}

/// The `LeaveField` switch template — an empty catalog key the author types over.
const fn leave_field_template() -> OnDeathEffect {
    OnDeathEffect::LeaveField {
        field: FieldKey::new(String::new()),
    }
}

/// The display label for the on-death variant combo — the closed vocabulary's authored
/// RON variant names.
const fn effect_label(effect: &OnDeathEffect) -> &'static str {
    match effect {
        OnDeathEffect::Explode { .. } => "Explode",
        OnDeathEffect::LeaveField { .. } => "LeaveField",
    }
}

/// The on-death variant [`ComboBox`](egui::ComboBox) over the closed two-variant
/// vocabulary — picking a DIFFERENT kind replaces the value with that kind's template.
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
