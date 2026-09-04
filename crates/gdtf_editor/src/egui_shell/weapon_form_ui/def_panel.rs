//! [`CollapsingHeader`](egui::CollapsingHeader)s so the author folds finished groups
//! whole authored record).
use bevy_egui::egui;
use gdtf_battle_sim::equipment::attachments::AttachmentRegistry;

use super::{lists, optionals, stats};
use crate::{
    egui_shell::{
        damage_edit::{self, DamageGroupFields},
        slots_edit,
    },
    weapon_form::WeaponDraft,
};

pub(crate) fn def_panel(
    ui: &mut egui::Ui,
    draft: &mut WeaponDraft,
    attachments: Option<&AttachmentRegistry>,
) {
    ui.heading("Weapon spec");
    ui.separator();

    let spec = draft.spec_mut();
    egui::CollapsingHeader::new("Stats")
        .default_open(true)
        .show(ui, |ui| stats::stats_group(ui, spec));
    egui::CollapsingHeader::new("Damage")
        .default_open(true)
        .show(ui, |ui| {
            damage_edit::damage_group(
                ui,
                "weapon",
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
        .show(ui, |ui| stats::handling_group(ui, spec));
    egui::CollapsingHeader::new("Magazine")
        .default_open(true)
        .show(ui, |ui| stats::magazine_group(ui, spec));
    egui::CollapsingHeader::new("Fire modes")
        .default_open(true)
        .show(ui, |ui| lists::fire_modes_list(ui, draft));

    let spec = draft.spec_mut();
    egui::CollapsingHeader::new("Slots")
        .default_open(true)
        .show(ui, |ui| {
            slots_edit::slots_list(ui, "weapon", &mut spec.slots);
        });
    egui::CollapsingHeader::new("Attachments")
        .default_open(true)
        .show(ui, |ui| {
            slots_edit::attachments_list(ui, "weapon", &mut spec.attachments, attachments);
        });
    egui::CollapsingHeader::new("Damage over time")
        .default_open(true)
        .show(ui, |ui| optionals::dot_form(ui, spec));
    egui::CollapsingHeader::new("On death")
        .default_open(true)
        .show(ui, |ui| optionals::on_death_form(ui, spec));
}
