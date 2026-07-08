//! The WEAPON tab's CENTRAL spec editor SKELETON (GTW-670 C2) — the full 18-field
//! [`WeaponSpec`](gdtf_battle_sim::weapon::WeaponSpec) form, grouped into collapsible
//! sections (the panel is long — the injuries / attachment scroll-stack shape, with
//! [`CollapsingHeader`](egui::CollapsingHeader)s so the author folds finished groups
//! away; every section defaults OPEN so a fresh look — and the A3 capture — shows the
//! whole authored record).
//!
//! The section BODIES live in the sibling per-concern files ([`stats`](super::stats) /
//! [`lists`](super::lists) / [`optionals`](super::optionals)) and — since GTW-671 — the
//! SHARED `egui_shell` widgets ([`damage_edit`](crate::egui_shell::damage_edit) for the
//! six fields the melee spec shares verbatim,
//! [`slots_edit`](crate::egui_shell::slots_edit) for the slot / attachment lists both
//! forms draw); this file owns only the grouping skeleton.

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

/// Draw the WEAPON-mode CENTRAL spec editor (GTW-670 C2): the nine collapsible groups
/// — stats (the ranged ballistics scalars), the SHARED damage group (GTW-671 — one
/// authoring surface with the melee form), handling (the trajectory combo + boolean
/// tags), magazine, fire modes, slots, attachments (registry-sourced key combos), and
/// the optional dot / on-death sub-forms. Each change folds through the matching sim
/// newtype's constructor — the model stays typed end to end (the attachment def-panel
/// pattern).
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
        .show(ui, |ui| lists::fire_modes_list(ui, spec));
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
