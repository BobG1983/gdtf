//! The INJURY tab's CENTRAL def editor (GTW-654 C1) — the authored
//! [`InjuryDef`](gdtf_battle_sim::injuries::InjuryDef) fields (display name /
//! category / severity / the three routed texts) plus the EFFECTS LIST over the
//! closed injury-effect palette: enum-driven rows (a variant combo + that
//! variant's payload fields) with add/remove — the attachment-effect-list
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

/// The three severities an injury may AUTHOR (`docs/authoring/injury-authoring.md`:
/// only `Minor` / `Major` / `Critical` are tabled — `None` is a graze and `Fatal`
/// is death via the existing gate, never authored) — the closed option set the
/// severity combo offers.
const TABLED_SEVERITIES: [Severity; 3] = [Severity::Minor, Severity::Major, Severity::Critical];

/// The seed each effect KIND switches to when the row's variant combo picks it —
/// one template per closed-palette variant, in the palette's declaration order.
/// The `Modify` template is the shared [`DEFAULT_EFFECT`]; `Bleeding` seeds the
/// smallest meaningful drain (1 HP/turn); `MovementCostMul` seeds the authoring
/// guide's light-slow example (`1.25` — `docs/authoring/injury-authoring.md` §1b;
/// the schema documents factors `>= 1.0`). All are starting points the author
/// immediately re-tunes, never shipped pins.
const EFFECT_TEMPLATES: [InjuryEffect; 4] = [
    DEFAULT_EFFECT,
    InjuryEffect::Bleeding {
        amount: BleedAmount::new(1),
    },
    InjuryEffect::DisableHand,
    InjuryEffect::MovementCostMul(MovementCostFactor::new(1.25)),
];

/// The display label for an effect row's variant combo — the closed palette's
/// authored RON variant names (`docs/authoring/injury-authoring.md` §1b).
const fn effect_label(effect: InjuryEffect) -> &'static str {
    match effect {
        InjuryEffect::Modify { .. } => "Modify",
        InjuryEffect::Bleeding { .. } => "Bleeding",
        InjuryEffect::DisableHand => "DisableHand",
        InjuryEffect::MovementCostMul(_) => "MovementCostMul",
    }
}

/// The display label for an [`InjuryCategory`] combo row — the authored RON
/// variant name (the pool axis, GTW-453).
const fn category_label(category: InjuryCategory) -> &'static str {
    match category {
        InjuryCategory::Head => "Head",
        InjuryCategory::Torso => "Torso",
        InjuryCategory::Arm => "Arm",
        InjuryCategory::Leg => "Leg",
    }
}

/// Draw the INJURY-mode CENTRAL def editor (GTW-654 C1): the display-name field,
/// the category / severity combos (closed enums), the three routed text fields,
/// and the effects list (one enum-driven row per authored effect, add/remove).
/// Each change folds through the matching sim newtype's constructor — the model
/// stays typed end to end (the gang attribute-grid pattern).
pub(crate) fn def_panel(ui: &mut egui::Ui, draft: &mut InjuryDraft) {
    ui.heading("Injury definition");
    ui.separator();

    identity_fields(ui, draft);
    text_fields(ui, draft);
    ui.separator();
    effects_list(ui, draft);
}

/// The identity row: the display name, the category pool, and the severity tier.
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

/// The three routed display texts (FCT popup / combat-log clause / inspect line).
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

/// The EFFECTS LIST — one enum-driven row per authored [`InjuryEffect`] (variant
/// combo + per-variant payload fields + Remove), plus the Add-effect button. A
/// remove press is folded in AFTER the loop (one structural edit per frame — the
/// gang member-list precedent; idempotent under the egui multipass re-run because
/// a click is a discrete event). Remove is disabled at one row: the `.injury.ron`
/// schema authors `effects` as `≥ 1`, so an empty list is unauthorable by
/// construction.
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

/// One row's variant [`ComboBox`](egui::ComboBox) over the closed palette —
/// picking a DIFFERENT kind replaces the row with that kind's template (the
/// current kind's row is a no-op, so an open combo never wipes a tuned payload).
/// Salted by row index so rows coexist.
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

/// One row's PER-VARIANT payload fields, editing the effect in place through the
/// sim payload newtypes' constructors. The drag clamps are the payload TYPES' own
/// ranges (`i8` / `u8` — the schema documents no tighter bounds), except the
/// movement factor's `>= 1.0` floor, which IS documented (the
/// [`MovementCostFactor`] "Hampered" convention: `1.0` identity, above slows;
/// below `1.0` is never authored).
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
        // Fieldless — the disabled side derives from the struck part at gain time.
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

/// The `Modify` row's [`StatTarget`] combo — all sixteen targets (the eight
/// attributes then the eight derived stats) in the sim's canonical
/// [`StatTarget::ALL`] order. Debug-formatted labels ARE the authored RON
/// identifiers (a fieldless serde variant serializes as its name). Salted by row
/// index so several `Modify` rows coexist.
fn stat_combo(ui: &mut egui::Ui, index: usize, stat: &mut StatTarget) {
    egui::ComboBox::from_id_salt(("injury_modify_stat", index))
        .selected_text(format!("{stat:?}"))
        .show_ui(ui, |ui| {
            for option in StatTarget::ALL {
                ui.selectable_value(stat, option, format!("{option:?}"));
            }
        });
}
