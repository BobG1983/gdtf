//! Shared weapon-form slot and attachment list widgets for `egui_shell`.
use bevy_egui::egui;
use gdtf_battle_sim::equipment::attachments::{
    AttachmentName, AttachmentRegistry, AttachmentSlot, FittedAttachments, SlotCapacity,
    WeaponSlots,
};

pub(in crate::egui_shell) fn slots_list(
    ui: &mut egui::Ui,
    salt: &'static str,
    slots: &mut WeaponSlots,
) {
    let mut remove: Option<usize> = None;
    for index in 0..slots.declarations().len() {
        let Some(&(slot, capacity)) = slots.declarations().get(index) else {
            continue;
        };
        let (mut slot, mut capacity) = (slot, capacity);
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt((salt, "slot", index))
                .selected_text(format!("{slot:?}"))
                .show_ui(ui, |ui| {
                    for option in AttachmentSlot::ALL {
                        ui.selectable_value(&mut slot, option, format!("{option:?}"));
                    }
                });
            ui.label("capacity");
            let mut count = *capacity;
            if ui.add(egui::DragValue::new(&mut count)).changed() {
                capacity = SlotCapacity::new(count);
            }
            if ui.button("Remove").clicked() {
                remove = Some(index);
            }
        });
        slots.set_slot(index, slot, capacity);
    }
    if let Some(index) = remove {
        slots.remove_slot(index);
    }
    if ui.button("Add slot").clicked() {
        slots.add_slot();
    }
}

pub(in crate::egui_shell) fn attachments_list(
    ui: &mut egui::Ui,
    salt: &'static str,
    authored_keys: &mut FittedAttachments,
    registry: Option<&AttachmentRegistry>,
) {
    let mut keys: Vec<&AttachmentName> = registry.map_or_else(Vec::new, |r| r.keys().collect());
    keys.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let mut remove: Option<usize> = None;
    for index in 0..authored_keys.len() {
        let Some(mut authored) = authored_keys.get(index).cloned() else {
            continue;
        };
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt((salt, "attachment", index))
                .selected_text(authored.as_str().to_owned())
                .show_ui(ui, |ui| {
                    for key in &keys {
                        let is_selected = authored == **key;
                        if ui.selectable_label(is_selected, key.as_str()).clicked() {
                            authored = (*key).clone();
                        }
                    }
                });
            if ui.button("Remove").clicked() {
                remove = Some(index);
            }
        });
        authored_keys.set(index, authored);
    }
    if let Some(index) = remove {
        authored_keys.remove(index);
    }
    let first = keys.first().map(|key| (*key).clone());
    match first {
        Some(seed) => {
            if ui.button("Add attachment").clicked() {
                authored_keys.add(seed);
            }
        }
        None => {
            ui.add_enabled(false, egui::Button::new("Add attachment"));
        }
    }
}
