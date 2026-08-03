//! The ATTACHMENT tab's CENTRAL item editor (GTW-669 C2) — the authored
//! with add/remove — the injury effects-list authoring shape drawn over the sim's own
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

pub(crate) fn def_panel(ui: &mut egui::Ui, draft: &mut AttachmentDraft) {
    ui.heading("Attachment item");
    ui.separator();

    identity_fields(ui, draft);
    ui.separator();
    effects_list(ui, draft);
}

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
        egui::ComboBox::from_id_salt("attachment_slot_combo")
            .selected_text(format!("{:?}", spec.slot))
            .show_ui(ui, |ui| {
                for option in AttachmentSlot::ALL {
                    ui.selectable_value(&mut spec.slot, option, format!("{option:?}"));
                }
            });
    });
}

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

fn drag_i32(ui: &mut egui::Ui, label: &str, value: i32, commit: impl FnOnce(i32)) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut edited = value;
        if ui.add(egui::DragValue::new(&mut edited)).changed() {
            commit(edited);
        }
    });
}

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
