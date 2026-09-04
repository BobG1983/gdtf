//! The FIELD tab's central editor: drain, channel, lifetime and the immunity tick boxes.

use std::num::NonZeroU8;

use bevy_egui::egui;
use gdtf_battle_sim::{
    armor::ArmorType,
    effects::fields::{FieldDamage, FieldDuration, FieldTurns},
    weapon::DamageType,
};

use crate::field_form::FieldDraft;

// The turn count a Turns duration currently names, or the minimum a fresh pick starts at.
fn held_turns(duration: FieldDuration) -> u8 {
    match duration {
        FieldDuration::Turns(turns) => turns.get(),
        FieldDuration::Permanent => FieldDraft::MIN_TURNS,
    }
}

// A finite duration for a count the form's own range keeps at or above the minimum.
fn finite(count: u8) -> Option<FieldDuration> {
    NonZeroU8::new(count.max(FieldDraft::MIN_TURNS))
        .map(|count| FieldDuration::Turns(FieldTurns::new(count)))
}

pub(crate) fn def_panel(ui: &mut egui::Ui, draft: &mut FieldDraft) {
    ui.heading("Field definition");
    ui.separator();

    key_field(ui, draft);
    damage_field(ui, draft);
    damage_type_combo(ui, draft);
    ui.separator();
    duration_pick(ui, draft);
    ui.separator();
    immunity_boxes(ui, draft);
}

fn key_field(ui: &mut egui::Ui, draft: &mut FieldDraft) {
    ui.horizontal(|ui| {
        ui.label("Field key");
        let mut key = draft.key().to_owned();
        if ui.text_edit_singleline(&mut key).changed() {
            draft.set_key(key);
        }
    });
}

fn damage_field(ui: &mut egui::Ui, draft: &mut FieldDraft) {
    ui.horizontal(|ui| {
        ui.label("Damage per tick");
        let mut damage = *draft.damage();
        if ui
            .add(egui::DragValue::new(&mut damage).range(FieldDraft::DAMAGE_RANGE))
            .changed()
        {
            draft.set_damage(FieldDamage::new(damage));
        }
    });
}

fn damage_type_combo(ui: &mut egui::Ui, draft: &mut FieldDraft) {
    ui.horizontal(|ui| {
        ui.label("Damage type");
        let current = draft.damage_type();
        let mut chosen = None;
        egui::ComboBox::from_id_salt("field_damage_type_combo")
            .selected_text(format!("{current:?}"))
            .show_ui(ui, |ui| {
                for option in DamageType::ALL {
                    if ui
                        .selectable_label(current == option, format!("{option:?}"))
                        .clicked()
                    {
                        chosen = Some(option);
                    }
                }
            });
        if let Some(option) = chosen {
            draft.set_damage_type(option);
        }
    });
}

fn duration_pick(ui: &mut egui::Ui, draft: &mut FieldDraft) {
    ui.label("Duration");
    let held = draft.duration();
    let is_permanent = matches!(held, FieldDuration::Permanent);
    ui.horizontal(|ui| {
        if ui.selectable_label(is_permanent, "Permanent").clicked() {
            draft.set_duration(FieldDuration::Permanent);
        }
        if ui.selectable_label(!is_permanent, "Turns").clicked()
            && let Some(duration) = finite(held_turns(held))
        {
            draft.set_duration(duration);
        }
    });
    if is_permanent {
        return;
    }
    ui.horizontal(|ui| {
        ui.label("Turns");
        let mut count = held_turns(draft.duration());
        let changed = ui
            .add(egui::DragValue::new(&mut count).range(FieldDraft::MIN_TURNS..=u8::MAX))
            .changed();
        if changed && let Some(duration) = finite(count) {
            draft.set_duration(duration);
        }
    });
}

fn immunity_boxes(ui: &mut egui::Ui, draft: &mut FieldDraft) {
    ui.label("Immune armor types (empty = drains everyone)");
    for armor_type in ArmorType::ALL {
        let mut listed = draft.is_immune(armor_type);
        if ui
            .checkbox(&mut listed, format!("{armor_type:?}"))
            .changed()
        {
            draft.toggle_immune_armor_type(armor_type);
        }
    }
}
