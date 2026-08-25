use bevy_egui::egui;
use gdtf_battle_sim::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart,
};

use crate::armor_form::ArmorDraft;

const TYPE_ORDER: [ArmorType; 7] = ArmorType::ALL;

const fn part_label(part: BodyPart) -> &'static str {
    match part {
        BodyPart::Head => "Head",
        BodyPart::Torso => "Torso",
        BodyPart::LeftArm => "L-Arm",
        BodyPart::RightArm => "R-Arm",
        BodyPart::LeftLeg => "L-Leg",
        BodyPart::RightLeg => "R-Leg",
    }
}

const fn type_label(armor_type: ArmorType) -> &'static str {
    match armor_type {
        ArmorType::Plated => "Plated",
        ArmorType::Refractive => "Refractive",
        ArmorType::Flak => "Flak",
        ArmorType::Void => "Void",
        ArmorType::Hazard => "Hazard",
        ArmorType::Reinforced => "Reinforced",
        ArmorType::Ceramic => "Ceramic",
    }
}

pub(crate) fn pieces_panel(ui: &mut egui::Ui, draft: &mut ArmorDraft) {
    ui.heading("Pieces");
    ui.separator();

    egui::Grid::new("armor_pieces_grid")
        .num_columns(6)
        .show(ui, |ui| {
            ui.label("Part");
            ui.label("Floor");
            ui.label("Protection");
            ui.label("Integrity");
            ui.label("Hardness");
            ui.label("Type");
            ui.end_row();

            for part in BodyPart::ALL {
                piece_row(ui, part, draft);
                ui.end_row();
            }
        });
}

fn piece_row(ui: &mut egui::Ui, part: BodyPart, draft: &mut ArmorDraft) {
    let piece = draft.piece_mut(part);
    ui.label(part_label(part));
    if let Some(v) = stat_drag(ui, *piece.floor, ArmorDraft::STAT_RANGE) {
        piece.floor = ArmorFloor::new(v);
    }
    if let Some(v) = stat_drag(ui, *piece.protection, ArmorDraft::STAT_RANGE) {
        piece.protection = ArmorProtection::new(v);
    }
    if let Some(v) = stat_drag(ui, *piece.integrity, ArmorDraft::INTEGRITY_RANGE) {
        piece.integrity = ArmorIntegrity::new(v);
    }
    if let Some(v) = stat_drag(ui, *piece.hardness, ArmorDraft::STAT_RANGE) {
        piece.hardness = ArmorHardness::new(v);
    }
    if let Some(t) = type_combo(ui, part, piece.armor_type) {
        piece.armor_type = t;
    }
}

fn stat_drag(ui: &mut egui::Ui, value: i32, range: core::ops::RangeInclusive<i32>) -> Option<i32> {
    let mut edited = value;
    let changed = ui
        .add(egui::DragValue::new(&mut edited).range(range))
        .changed();
    changed.then_some(edited)
}

fn type_combo(ui: &mut egui::Ui, part: BodyPart, current: ArmorType) -> Option<ArmorType> {
    let mut chosen = None;
    egui::ComboBox::from_id_salt(("armor_piece_type", part.index()))
        .selected_text(type_label(current))
        .show_ui(ui, |ui| {
            for option in TYPE_ORDER {
                if ui
                    .selectable_label(current == option, type_label(option))
                    .clicked()
                {
                    chosen = Some(option);
                }
            }
        });
    chosen
}
