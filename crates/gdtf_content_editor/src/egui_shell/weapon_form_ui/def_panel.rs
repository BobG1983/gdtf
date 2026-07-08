//! The WEAPON tab's CENTRAL spec editor SKELETON (GTW-670 C2) — the full 18-field
//! [`WeaponSpec`](gdtf_battle_sim::weapon::WeaponSpec) form, grouped into collapsible
//! sections (the panel is long — the injuries / attachment scroll-stack shape, with
//! [`CollapsingHeader`](egui::CollapsingHeader)s so the author folds finished groups
//! away; every section defaults OPEN so a fresh look — and the A3 capture — shows the
//! whole authored record).
//!
//! The section BODIES live in the sibling per-concern files ([`stats`](super::stats) /
//! [`lists`](super::lists) / [`optionals`](super::optionals)); this file owns only the
//! grouping skeleton.

use bevy_egui::egui;
use gdtf_battle_sim::equipment::attachments::AttachmentRegistry;

use super::{lists, optionals, stats};
use crate::weapon_form::WeaponDraft;

/// Draw the WEAPON-mode CENTRAL spec editor (GTW-670 C2): the eight collapsible groups
/// — stats, handling (the closed-vocabulary combos + boolean tags), magazine, fire
/// modes, slots, attachments (registry-sourced key combos), and the optional dot /
/// on-death sub-forms. Each change folds through the matching sim newtype's
/// constructor — the model stays typed end to end (the attachment def-panel pattern).
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
        .show(ui, |ui| lists::slots_list(ui, spec));
    egui::CollapsingHeader::new("Attachments")
        .default_open(true)
        .show(ui, |ui| lists::attachments_list(ui, spec, attachments));
    egui::CollapsingHeader::new("Damage over time")
        .default_open(true)
        .show(ui, |ui| optionals::dot_form(ui, spec));
    egui::CollapsingHeader::new("On death")
        .default_open(true)
        .show(ui, |ui| optionals::on_death_form(ui, spec));
}
