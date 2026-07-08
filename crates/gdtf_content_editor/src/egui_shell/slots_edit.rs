//! The REUSABLE **slot-declaration + attachment-key list editors** (GTW-671 C2 — the
//! GTW-670 weapon-form widgets, lifted to a shared `egui_shell` sibling): one authoring
//! surface for a weapon's `slots:` declarations and its `attachments:` key list, drawn
//! by BOTH the ranged WEAPON form and the melee MELEE-WEAPON form (the contract's "same
//! widget as ranged" clause — the [`fire_mode_edit`](super::fire_mode_edit) /
//! [`damage_edit`](super::damage_edit) shared-leaf precedent).
//!
//! Both widgets edit the SHARED field types directly (`WeaponSlots` /
//! `Vec<AttachmentName>` — identical on both spec records), so neither form forks the
//! list semantics. The [`WeaponSlots`] sim newtype is construct-only (a private inner
//! with a read accessor), so the slots list edits by PROJECTING the authored
//! declarations out, mutating them, and folding back through the SAME constructor the
//! loader's deserialize uses. One structural edit (add / remove) folds in per frame
//! (the gang member-list precedent; a click is a discrete event, so the fold is
//! idempotent under the egui multipass re-run).

use bevy_egui::egui;
use gdtf_battle_sim::equipment::attachments::{
    AttachmentName, AttachmentRegistry, AttachmentSlot, SlotCapacity, WeaponSlots,
};

/// The SLOTS list — one row per authored `(slot, capacity)` declaration: the slot combo
/// over the closed 5-slot palette ([`AttachmentSlot::ALL`] — the melee-typical
/// `Counterweight` / `Pommel` author naturally since the palette is closed, GTW-671 C2)
/// and the capacity drag (the `u8` type's own range — no documented tighter bound),
/// plus add/remove. An Add seeds the palette's first slot at capacity 1 (a deliberately
/// inert starting pick — the attachment seed rule, immediately re-tuned). `salt` is the
/// calling form's id prefix so both forms' rows coexist.
pub(in crate::egui_shell) fn slots_list(
    ui: &mut egui::Ui,
    salt: &'static str,
    slots: &mut WeaponSlots,
) {
    let mut declarations: Vec<(AttachmentSlot, SlotCapacity)> = slots.declarations().to_vec();
    let mut remove: Option<usize> = None;
    for (index, (slot, capacity)) in declarations.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt((salt, "slot", index))
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
    *slots = WeaponSlots::new(declarations);
}

/// The ATTACHMENTS list — one row per authored [`AttachmentName`] key, each a combo
/// over the SORTED registry keys only (GTW-670/671 C2: a dangling key is UNAUTHORABLE
/// via the form — the GTW-669 registration is what makes the option source live), plus
/// add/remove. Add is disabled while the registry is absent / empty (nothing legal to
/// reference — the injury weighting-row precedent); a key already authored on disk that
/// no longer resolves still DISPLAYS (the author's file is the truth) and surfaces
/// through the GTW-669 validation edge (which chains the MELEE registry too), not
/// through this widget. `salt` is the calling form's id prefix so both forms' rows
/// coexist.
pub(in crate::egui_shell) fn attachments_list(
    ui: &mut egui::Ui,
    salt: &'static str,
    authored_keys: &mut Vec<AttachmentName>,
    registry: Option<&AttachmentRegistry>,
) {
    let mut keys: Vec<&AttachmentName> = registry.map_or_else(Vec::new, |r| r.keys().collect());
    keys.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let mut remove: Option<usize> = None;
    for (index, authored) in authored_keys.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt((salt, "attachment", index))
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
        authored_keys.remove(index);
    }
    let first = keys.first().map(|key| (*key).clone());
    match first {
        Some(seed) => {
            if ui.button("Add attachment").clicked() {
                authored_keys.push(seed);
            }
        }
        // Registry absent / empty: nothing legal to reference, so Add is disabled
        // (dangling-by-construction impossible).
        None => {
            ui.add_enabled(false, egui::Button::new("Add attachment"));
        }
    }
}
