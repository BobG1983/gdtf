//! The MELEE tab's CENTRAL spec editor SKELETON (GTW-671 C2) — the full
//! [`MeleeWeaponSpec`](gdtf_battle_sim::weapon::MeleeWeaponSpec) form, grouped into
//! collapsible sections (the GTW-670 weapon def-panel shape, with
//! [`CollapsingHeader`](egui::CollapsingHeader)s defaulting OPEN so a fresh look — and
//! the A3 capture — shows the whole authored record).
//!
//! The section BODIES are the SHARED `egui_shell` widgets
//! ([`damage_edit`](crate::egui_shell::damage_edit) for the six fields the ranged spec
//! shares verbatim, [`slots_edit`](crate::egui_shell::slots_edit) for the slot /
//! attachment lists) plus the melee-only [`fight_modes`](super::fight_modes) rows and
//! the handling group below; this file owns the grouping skeleton and the two
//! melee-only handling controls.

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

/// Draw the MELEE-WEAPON-mode CENTRAL spec editor (GTW-671 C2): the five collapsible
/// groups — the SHARED damage group (one authoring surface with the ranged form),
/// handling (the melee-only reach drag + the shared shove tag), fight modes (the melee
/// sibling of the fire-mode rows), slots, and attachments (registry-sourced key combos
/// — the same shared widgets the ranged form draws). Each change folds through the
/// matching sim newtype's constructor — the model stays typed end to end (the
/// attachment / weapon def-panel pattern). NO ranged-only field appears: the melee spec
/// has no spread / magazine / trajectory / dot / on-death, so the form draws none (the
/// GTW-671 zero-bleed-through fidelity clause).
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

/// The melee HANDLING group — the `reach` drag and the `shove` tag (GTW-671 C2). The
/// reach drag clamps to `1..=u16::MAX`: the contract's "min 1 per the default's
/// semantics" — [`Reach::DEFAULT`] documents `1` (the adjacent-cell strike) as BOTH the
/// serde-default and the defensible melee minimum, so a zero-reach weapon (which could
/// strike nothing) is unauthorable via the form. The upper bound is the `u16` type's
/// own range (no documented tighter bound — the GTW-479 ruling).
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
