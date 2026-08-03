//! Shared weapon-form slot and attachment list widgets for `egui_shell`.
use bevy_egui::egui;
use gdtf_battle_sim::equipment::attachments::{
    AttachmentName, AttachmentRegistry, AttachmentSlot, SlotCapacity, WeaponSlots,
};

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
    *slots = WeaponSlots::new(declarations);
}

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
        None => {
            ui.add_enabled(false, egui::Button::new("Add attachment"));
        }
    }
}
