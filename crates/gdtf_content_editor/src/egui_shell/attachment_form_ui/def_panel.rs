//! The ATTACHMENT tab's CENTRAL item editor (GTW-669 C2) — the authored
//! [`AttachmentSpec`](gdtf_battle_sim::equipment::attachments::AttachmentSpec) fields
//! (display name / the closed 5-slot mount) plus the EFFECTS LIST over the closed
//! 13-effect palette: enum-driven rows (a variant combo + that variant's payload fields)
//! with add/remove — the injury effects-list authoring shape drawn over the sim's own
//! [`AttachmentEffect`] enum, with the `GainFireMode` payload drawing the SHARED
//! [`fire_mode_edit`](crate::egui_shell::fire_mode_edit) row widget. An EMPTY effects
//! list is LEGAL (the schema's cosmetic identity), so Remove is never disabled — the
//! deliberate contrast with the injuries form's `effects >= 1` floor.

use bevy_egui::egui;
use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect, ReloadTimeScale, WeaponBraceBonus},
    equipment::attachments::AttachmentSlot,
    weapon::{
        DamageType, FatalBias, FireModeSpec, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};

use crate::{attachment_form::AttachmentDraft, egui_shell::fire_mode_edit};

/// The seed each effect KIND switches to when a row's variant combo picks it — one
/// template per closed-palette variant, in the palette's declaration order. Every
/// payload seeds its DOCUMENTED IDENTITY where one exists ([`WeaponBraceBonus::none`],
/// the `ReloadTime` `1.0` identity, [`DamageType`]'s documented `Kinetic` default, the
/// sim's structural single-shot fire mode) or the zero magnitude otherwise — starting
/// points the author immediately re-tunes, never shipped pins.
const EFFECT_TEMPLATES: [AttachmentEffect; 13] = [
    AttachmentEffect::Aim(AimDelta::new(0.0)),
    AttachmentEffect::Stability(WeaponBraceBonus::none()),
    AttachmentEffect::GainFireMode(FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.0),
        ModeShots::new(1),
    )),
    AttachmentEffect::ExtraAmmo(MagazineSize::new(0)),
    AttachmentEffect::ReloadTime(ReloadTimeScale::new(1.0)),
    AttachmentEffect::Silence,
    AttachmentEffect::Penetration(WeaponPunch::new(0)),
    AttachmentEffect::DamageTypeOverride(DamageType::Kinetic),
    AttachmentEffect::Damage(WeaponDamage::new(0)),
    AttachmentEffect::Shred(WeaponShred::new(0)),
    AttachmentEffect::FatalBias(FatalBias::new(0.0)),
    AttachmentEffect::Brace,
    AttachmentEffect::Shove,
];

/// The display label for an effect row's variant combo — the closed palette's authored
/// RON variant names (the injury `effect_label` convention).
const fn effect_label(effect: &AttachmentEffect) -> &'static str {
    match effect {
        AttachmentEffect::Aim(_) => "Aim",
        AttachmentEffect::Stability(_) => "Stability",
        AttachmentEffect::GainFireMode(_) => "GainFireMode",
        AttachmentEffect::ExtraAmmo(_) => "ExtraAmmo",
        AttachmentEffect::ReloadTime(_) => "ReloadTime",
        AttachmentEffect::Silence => "Silence",
        AttachmentEffect::Penetration(_) => "Penetration",
        AttachmentEffect::DamageTypeOverride(_) => "DamageTypeOverride",
        AttachmentEffect::Damage(_) => "Damage",
        AttachmentEffect::Shred(_) => "Shred",
        AttachmentEffect::FatalBias(_) => "FatalBias",
        AttachmentEffect::Brace => "Brace",
        AttachmentEffect::Shove => "Shove",
    }
}

/// Draw the ATTACHMENT-mode CENTRAL item editor (GTW-669 C2): the display-name field,
/// the closed 5-slot combo, and the effects list (one enum-driven row per authored
/// effect over the closed 13-effect palette, add/remove — empty is legal). Each change
/// folds through the matching sim newtype's constructor — the model stays typed end to
/// end (the injury def-panel pattern).
pub(crate) fn def_panel(ui: &mut egui::Ui, draft: &mut AttachmentDraft) {
    ui.heading("Attachment item");
    ui.separator();

    identity_fields(ui, draft);
    ui.separator();
    effects_list(ui, draft);
}

/// The identity row: the human-facing display name and the single mount slot.
fn identity_fields(ui: &mut egui::Ui, draft: &mut AttachmentDraft) {
    let spec = draft.spec_mut();
    ui.horizontal(|ui| {
        ui.label("Display name");
        let mut name = spec.display_name.as_str().to_owned();
        if ui.text_edit_singleline(&mut name).changed() {
            spec.display_name = WeaponName::new(name);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Slot");
        // The closed 5-slot palette is the sim's own canonical enumeration
        // (`AttachmentSlot::ALL` — GTW-670 hoisted it beside `DamageType::ALL`, so the
        // WEAPON form's slots rows and this combo share one source).
        egui::ComboBox::from_id_salt("attachment_slot_combo")
            .selected_text(format!("{:?}", spec.slot))
            .show_ui(ui, |ui| {
                for option in AttachmentSlot::ALL {
                    ui.selectable_value(&mut spec.slot, option, format!("{option:?}"));
                }
            });
    });
}

/// The EFFECTS LIST — one enum-driven row per authored [`AttachmentEffect`] (variant
/// combo + per-variant payload fields + Remove), plus the Add-effect button. A remove
/// press is folded in AFTER the loop (one structural edit per frame — the gang
/// member-list precedent; idempotent under the egui multipass re-run because a click is
/// a discrete event). Remove is ALWAYS enabled and Add seeds the palette's first
/// template: an EMPTY list is the schema's legal cosmetic identity (GTW-669 C2 — no
/// `effects >= 1` floor, the deliberate contrast with the injuries form).
fn effects_list(ui: &mut egui::Ui, draft: &mut AttachmentDraft) {
    ui.label("Effects (empty = cosmetic)");
    let effects = &mut draft.spec_mut().effects;
    let mut remove: Option<usize> = None;
    for (index, effect) in effects.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                variant_combo(ui, index, effect);
                if ui.button("Remove").clicked() {
                    remove = Some(index);
                }
            });
            payload_fields(ui, index, effect);
        });
    }
    if let Some(index) = remove {
        effects.remove(index);
    }
    if ui.button("Add effect").clicked() {
        effects.push(EFFECT_TEMPLATES[0].clone());
    }
}

/// One row's variant [`ComboBox`](egui::ComboBox) over the closed 13-effect palette —
/// picking a DIFFERENT kind replaces the row with that kind's template (the current
/// kind's row is a no-op, so an open combo never wipes a tuned payload). Salted by row
/// index so rows coexist.
fn variant_combo(ui: &mut egui::Ui, index: usize, effect: &mut AttachmentEffect) {
    let current = effect_label(effect);
    egui::ComboBox::from_id_salt(("attachment_effect_kind", index))
        .selected_text(current)
        .show_ui(ui, |ui| {
            for template in EFFECT_TEMPLATES {
                let label = effect_label(&template);
                if ui.selectable_label(current == label, label).clicked() && current != label {
                    *effect = template;
                }
            }
        });
}

/// One row's PER-VARIANT payload fields, editing the effect in place through the sim
/// payload newtypes' constructors. NO drag clamp is invented: every attachment magnitude
/// is documented tuning-open (the GTW-479 ruling), so the drags span their payload
/// TYPES' own ranges. The fieldless variants (`Silence` / `Brace` / `Shove` — the
/// boolean weapon tags) draw nothing.
fn payload_fields(ui: &mut egui::Ui, index: usize, effect: &mut AttachmentEffect) {
    match effect {
        AttachmentEffect::Aim(delta) => {
            drag_f32(ui, "accuracy +", 0.05, **delta, |v| {
                *delta = AimDelta::new(v);
            });
        }
        AttachmentEffect::Stability(bonus) => {
            drag_f32(ui, "stability pts", 0.5, **bonus, |v| {
                *bonus = WeaponBraceBonus::new(v);
            });
        }
        // The nested fire-mode row — the SHARED widget the GTW-670 weapon forms reuse.
        AttachmentEffect::GainFireMode(mode) => fire_mode_edit::fire_mode_row(ui, index, mode),
        AttachmentEffect::ExtraAmmo(size) => {
            ui.horizontal(|ui| {
                ui.label("rounds +");
                let mut value = **size;
                if ui.add(egui::DragValue::new(&mut value)).changed() {
                    *size = MagazineSize::new(value);
                }
            });
        }
        AttachmentEffect::ReloadTime(scale) => {
            drag_f32(ui, "×reload TU", 0.05, **scale, |v| {
                *scale = ReloadTimeScale::new(v);
            });
        }
        AttachmentEffect::Silence | AttachmentEffect::Brace | AttachmentEffect::Shove => {}
        AttachmentEffect::Penetration(punch) => {
            drag_i32(ui, "punch +", **punch, |v| *punch = WeaponPunch::new(v));
        }
        AttachmentEffect::DamageTypeOverride(ty) => damage_type_combo(ui, index, ty),
        AttachmentEffect::Damage(damage) => {
            drag_i32(ui, "damage +", **damage, |v| *damage = WeaponDamage::new(v));
        }
        AttachmentEffect::Shred(shred) => {
            drag_i32(ui, "shred +", **shred, |v| *shred = WeaponShred::new(v));
        }
        AttachmentEffect::FatalBias(bias) => {
            drag_f32(ui, "fatal bias +", 0.05, **bias, |v| {
                *bias = FatalBias::new(v);
            });
        }
    }
}

/// A labelled `f32` payload drag on its own row, committing through `commit` only on a
/// change (the injury payload-drag shape, factored since the palette has five float
/// payloads).
fn drag_f32(ui: &mut egui::Ui, label: &str, speed: f64, value: f32, commit: impl FnOnce(f32)) {
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

/// A labelled `i32` payload drag on its own row — the signed-magnitude twin of
/// [`drag_f32`] (punch / damage / shred share the sim's honest-signed `i32`).
fn drag_i32(ui: &mut egui::Ui, label: &str, value: i32, commit: impl FnOnce(i32)) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut edited = value;
        if ui.add(egui::DragValue::new(&mut edited)).changed() {
            commit(edited);
        }
    });
}

/// The `DamageTypeOverride` row's [`DamageType`] combo — the seven wheel nodes in the
/// sim's canonical [`DamageType::ALL`] order. Debug-formatted labels ARE the authored
/// RON identifiers (a fieldless serde variant serializes as its name — the injury
/// stat-combo convention). Salted by row index so several override rows coexist.
fn damage_type_combo(ui: &mut egui::Ui, index: usize, ty: &mut DamageType) {
    ui.horizontal(|ui| {
        ui.label("emits");
        egui::ComboBox::from_id_salt(("attachment_damage_type", index))
            .selected_text(format!("{ty:?}"))
            .show_ui(ui, |ui| {
                for option in DamageType::ALL {
                    ui.selectable_value(ty, option, format!("{option:?}"));
                }
            });
    });
}
