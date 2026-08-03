//! The INJURY tab's CENTRAL def editor (GTW-654 C1) — the authored
//! authoring shape drawn over the sim's own [`InjuryEffect`] enum.

use bevy_egui::egui;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{
        BleedAmount, InjuryEffect, InjuryName, InspectText, LogText, MovementCostFactor, PopupText,
        StatDelta, StatTarget,
    },
    severity::Severity,
};

use crate::injury_form::{DEFAULT_EFFECT, InjuryDraft};

const TABLED_SEVERITIES: [Severity; 3] = [Severity::Minor, Severity::Major, Severity::Critical];

const EFFECT_TEMPLATES: [InjuryEffect; 4] = [
    DEFAULT_EFFECT,
    InjuryEffect::Bleeding {
        amount: BleedAmount::new(1),
    },
    InjuryEffect::DisableHand,
    InjuryEffect::MovementCostMul(MovementCostFactor::new(1.25)),
];

const fn effect_label(effect: InjuryEffect) -> &'static str {
    match effect {
        InjuryEffect::Modify { .. } => "Modify",
        InjuryEffect::Bleeding { .. } => "Bleeding",
        InjuryEffect::DisableHand => "DisableHand",
        InjuryEffect::MovementCostMul(_) => "MovementCostMul",
    }
}

const fn category_label(category: InjuryCategory) -> &'static str {
    match category {
        InjuryCategory::Head => "Head",
        InjuryCategory::Torso => "Torso",
        InjuryCategory::Arm => "Arm",
        InjuryCategory::Leg => "Leg",
    }
}

pub(crate) fn def_panel(ui: &mut egui::Ui, draft: &mut InjuryDraft) {
    ui.heading("Injury definition");
    ui.separator();

    identity_fields(ui, draft);
    text_fields(ui, draft);
    ui.separator();
    effects_list(ui, draft);
}

fn identity_fields(ui: &mut egui::Ui, draft: &mut InjuryDraft) {
    let def = draft.def_mut();
    ui.horizontal(|ui| {
        ui.label("Display name");
        let mut name = def.name.as_str().to_owned();
        if ui.text_edit_singleline(&mut name).changed() {
            def.name = InjuryName::new(name);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Category");
        egui::ComboBox::from_id_salt("injury_category_combo")
            .selected_text(category_label(def.category))
            .show_ui(ui, |ui| {
                for option in InjuryCategory::ALL {
                    ui.selectable_value(&mut def.category, option, category_label(option));
                }
            });
        ui.label("Severity");
        egui::ComboBox::from_id_salt("injury_severity_combo")
            .selected_text(format!("{:?}", def.severity))
            .show_ui(ui, |ui| {
                for option in TABLED_SEVERITIES {
                    ui.selectable_value(&mut def.severity, option, format!("{option:?}"));
                }
            });
    });
}

fn text_fields(ui: &mut egui::Ui, draft: &mut InjuryDraft) {
    let def = draft.def_mut();
    ui.horizontal(|ui| {
        ui.label("Popup text");
        let mut text = def.popup_text.as_str().to_owned();
        if ui.text_edit_singleline(&mut text).changed() {
            def.popup_text = PopupText::new(text);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Log text");
        let mut text = def.log_text.as_str().to_owned();
        if ui.text_edit_singleline(&mut text).changed() {
            def.log_text = LogText::new(text);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Inspect text");
        let mut text = def.inspect_text.as_str().to_owned();
        if ui.text_edit_singleline(&mut text).changed() {
            def.inspect_text = InspectText::new(text);
        }
    });
}

fn effects_list(ui: &mut egui::Ui, draft: &mut InjuryDraft) {
    ui.label("Effects");
    let effects = &mut draft.def_mut().effects;
    let removable = effects.len() > 1;
    let mut remove: Option<usize> = None;
    for (index, effect) in effects.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            variant_combo(ui, index, effect);
            payload_fields(ui, index, effect);
            if ui
                .add_enabled(removable, egui::Button::new("Remove"))
                .clicked()
            {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        effects.remove(index);
    }
    if ui.button("Add effect").clicked() {
        effects.push(DEFAULT_EFFECT);
    }
}

fn variant_combo(ui: &mut egui::Ui, index: usize, effect: &mut InjuryEffect) {
    let current = effect_label(*effect);
    egui::ComboBox::from_id_salt(("injury_effect_kind", index))
        .selected_text(current)
        .show_ui(ui, |ui| {
            for template in EFFECT_TEMPLATES {
                let label = effect_label(template);
                if ui.selectable_label(current == label, label).clicked() && current != label {
                    *effect = template;
                }
            }
        });
}

fn payload_fields(ui: &mut egui::Ui, index: usize, effect: &mut InjuryEffect) {
    match effect {
        InjuryEffect::Modify { stat, amount } => {
            stat_combo(ui, index, stat);
            let mut value = i32::from(amount.raw());
            if ui
                .add(
                    egui::DragValue::new(&mut value).range(i32::from(i8::MIN)..=i32::from(i8::MAX)),
                )
                .changed()
            {
                *amount = StatDelta::new(i8::try_from(value).unwrap_or_default());
            }
        }
        InjuryEffect::Bleeding { amount } => {
            ui.label("HP/turn");
            let mut value = u32::from(amount.raw());
            if ui
                .add(egui::DragValue::new(&mut value).range(0..=u32::from(u8::MAX)))
                .changed()
            {
                *amount = BleedAmount::new(u8::try_from(value).unwrap_or_default());
            }
        }
        InjuryEffect::DisableHand => {}
        InjuryEffect::MovementCostMul(factor) => {
            ui.label("×TU/step");
            let mut value = factor.raw();
            if ui
                .add(
                    egui::DragValue::new(&mut value)
                        .speed(0.05)
                        .range(1.0..=f32::MAX),
                )
                .changed()
            {
                *factor = MovementCostFactor::new(value);
            }
        }
    }
}

fn stat_combo(ui: &mut egui::Ui, index: usize, stat: &mut StatTarget) {
    egui::ComboBox::from_id_salt(("injury_modify_stat", index))
        .selected_text(format!("{stat:?}"))
        .show_ui(ui, |ui| {
            for option in StatTarget::ALL {
                ui.selectable_value(stat, option, format!("{option:?}"));
            }
        });
}
