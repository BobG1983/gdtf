//! The WEAPON form's three LIST editors (GTW-670 C2) — the fire-mode rows (drawing the
//! SHARED [`fire_mode_edit`](crate::egui_shell::fire_mode_edit) widget), the slot
//! declarations, and the attachment key rows.
//!
//! The [`FireMode`] / [`WeaponSlots`] sim newtypes are construct-only (private inners
//! with read accessors), so each list edits by PROJECTING the authored list out,
//! mutating it, and folding it back through the SAME constructor the loader's
//! deserialize uses — the model stays the loader schema end to end. One structural edit
//! (add / remove) folds in per frame (the gang member-list precedent; a click is a
//! discrete event, so the fold is idempotent under the egui multipass re-run).

use bevy_egui::egui;
use gdtf_battle_sim::{
    equipment::attachments::{
        AttachmentName, AttachmentRegistry, AttachmentSlot, SlotCapacity, WeaponSlots,
    },
    weapon::{FireMode, FireModeSpec, WeaponSpec},
};

use crate::{egui_shell::fire_mode_edit, weapon_form::structural_single_mode};

/// The FIRE-MODE list — one row per authored [`FireModeSpec`] via the SHARED
/// [`fire_mode_edit::fire_mode_row`] widget (GTW-670 C2 — the same authoring surface
/// the ATTACHMENT mode's `GainFireMode` rows draw, never a fork) plus Remove, and the
/// Add-mode button seeding the structural single-shot template. Remove is DISABLED at
/// one mode: the [`FireMode`] invariant documents "a well-authored weapon lists at
/// least one mode", so a mode-less weapon is unauthorable by construction (the
/// injuries `effects >= 1` floor precedent).
pub(super) fn fire_modes_list(ui: &mut egui::Ui, spec: &mut WeaponSpec) {
    let mut modes: Vec<FireModeSpec> = spec.fire_mode.to_vec();
    let single_mode = modes.len() == 1;
    let mut remove: Option<usize> = None;
    for (index, mode) in modes.iter_mut().enumerate() {
        ui.group(|ui| {
            fire_mode_edit::fire_mode_row(ui, index, mode);
            if ui
                .add_enabled(!single_mode, egui::Button::new("Remove mode"))
                .clicked()
            {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        modes.remove(index);
    }
    if ui.button("Add fire mode").clicked() {
        modes.push(structural_single_mode());
    }
    // Fold the edited list back through the loader's own constructor.
    spec.fire_mode = FireMode::new(modes);
}

/// The SLOTS list — one row per authored `(slot, capacity)` declaration: the slot combo
/// over the closed 5-slot palette ([`AttachmentSlot::ALL`]) + the capacity drag (the
/// `u8` type's own range — no documented tighter bound), plus add/remove (GTW-670 C2).
/// An Add seeds the palette's first slot at capacity 1 (a deliberately inert starting
/// pick — the attachment seed rule, immediately re-tuned).
pub(super) fn slots_list(ui: &mut egui::Ui, spec: &mut WeaponSpec) {
    let mut declarations: Vec<(AttachmentSlot, SlotCapacity)> = spec.slots.declarations().to_vec();
    let mut remove: Option<usize> = None;
    for (index, (slot, capacity)) in declarations.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt(("weapon_slot", index))
                .selected_text(format!("{slot:?}"))
                .show_ui(ui, |ui| {
                    for option in AttachmentSlot::ALL {
                        ui.selectable_value(slot, option, format!("{option:?}"));
                    }
                });
            ui.label("capacity");
            let mut count = **capacity;
            if ui.add(egui::DragValue::new(&mut count)).changed() {
                *capacity = SlotCapacity::new(count);
            }
            if ui.button("Remove").clicked() {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        declarations.remove(index);
    }
    if ui.button("Add slot").clicked() {
        declarations.push((AttachmentSlot::Muzzle, SlotCapacity::new(1)));
    }
    // Fold the edited declarations back through the loader's own constructor.
    spec.slots = WeaponSlots::new(declarations);
}

/// The ATTACHMENTS list — one row per authored [`AttachmentName`] key, each a combo
/// over the SORTED registry keys only (GTW-670 C2: a dangling key is UNAUTHORABLE via
/// the form — the GTW-669 registration is what makes the option source live), plus
/// add/remove. Add is disabled while the registry is absent / empty (nothing legal to
/// reference — the injury weighting-row precedent); a key already authored on disk that
/// no longer resolves still DISPLAYS (the author's file is the truth) and surfaces
/// through the GTW-669 validation edge, not through this widget.
pub(super) fn attachments_list(
    ui: &mut egui::Ui,
    spec: &mut WeaponSpec,
    registry: Option<&AttachmentRegistry>,
) {
    let mut keys: Vec<&AttachmentName> = registry.map_or_else(Vec::new, |r| r.keys().collect());
    keys.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let mut remove: Option<usize> = None;
    for (index, authored) in spec.attachments.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt(("weapon_attachment", index))
                .selected_text(authored.as_str().to_owned())
                .show_ui(ui, |ui| {
                    for key in &keys {
                        let is_selected = authored == *key;
                        if ui.selectable_label(is_selected, key.as_str()).clicked() {
                            *authored = (*key).clone();
                        }
                    }
                });
            if ui.button("Remove").clicked() {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        spec.attachments.remove(index);
    }
    let first = keys.first().map(|key| (*key).clone());
    match first {
        Some(seed) => {
            if ui.button("Add attachment").clicked() {
                spec.attachments.push(seed);
            }
        }
        // Registry absent / empty: nothing legal to reference, so Add is disabled
        // (dangling-by-construction impossible).
        None => {
            ui.add_enabled(false, egui::Button::new("Add attachment"));
        }
    }
}
