//! the A3 capture — shows the whole authored record).
use bevy_egui::egui;
use gdtf_battle_sim::{
    equipment::attachments::AttachmentRegistry,
    weapon::{MeleeWeaponSpec, Reach, Shove},
};

use super::fight_modes;
use crate::{
    egui_shell::{
        damage_edit::{self, DamageGroupFields},
        slots_edit,
    },
    melee_weapon_form::MeleeWeaponDraft,
};

pub(crate) fn def_panel(
    ui: &mut egui::Ui,
    draft: &mut MeleeWeaponDraft,
    attachments: Option<&AttachmentRegistry>,
) {
    ui.heading("Melee weapon spec");
    ui.separator();

    let spec = draft.spec_mut();
    egui::CollapsingHeader::new("Damage")
        .default_open(true)
        .show(ui, |ui| {
            damage_edit::damage_group(
                ui,
                "melee_weapon",
                DamageGroupFields {
                    damage:      &mut spec.damage,
                    punch:       &mut spec.punch,
                    shred:       &mut spec.shred,
                    damage_type: &mut spec.damage_type,
                    fatal_bias:  &mut spec.fatal_bias,
                    handedness:  &mut spec.handedness,
                },
            );
        });
    egui::CollapsingHeader::new("Handling")
        .default_open(true)
        .show(ui, |ui| handling_group(ui, spec));
    egui::CollapsingHeader::new("Fight modes")
        .default_open(true)
        .show(ui, |ui| fight_modes::fight_modes_list(ui, spec));
    egui::CollapsingHeader::new("Slots")
        .default_open(true)
        .show(ui, |ui| {
            slots_edit::slots_list(ui, "melee_weapon", &mut spec.slots);
        });
    egui::CollapsingHeader::new("Attachments")
        .default_open(true)
        .show(ui, |ui| {
            slots_edit::attachments_list(ui, "melee_weapon", &mut spec.attachments, attachments);
        });
}

fn handling_group(ui: &mut egui::Ui, spec: &mut MeleeWeaponSpec) {
    ui.horizontal(|ui| {
        ui.label("reach");
        let mut reach = *spec.reach;
        if ui
            .add(egui::DragValue::new(&mut reach).range(1..=u16::MAX))
            .changed()
        {
            spec.reach = Reach::new(reach.max(1));
        }
    });
    let mut shove = *spec.shove;
    if ui
        .checkbox(&mut shove, "shove (knock-back on connect)")
        .changed()
    {
        spec.shove = Shove::new(shove);
    }
}
