//! Injury weighting panel; combo only offers resolving keys.
use std::path::Path;

use bevy_egui::egui;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{
        DamageContext, InjuryName, InjuryRegistry, InjuryTables, InjuryWeight, WeightedInjuryEntry,
    },
    severity::Severity,
};

use crate::{injury_form::WeightingDraft, save_record::LastSaveRecord};

const fn category_label(category: InjuryCategory) -> &'static str {
    match category {
        InjuryCategory::Head => "Head",
        InjuryCategory::Torso => "Torso",
        InjuryCategory::Arm => "Arm",
        InjuryCategory::Leg => "Leg",
    }
}

const fn context_label(context: DamageContext) -> &'static str {
    match context {
        DamageContext::Ranged => "Ranged",
        DamageContext::Melee => "Melee",
        DamageContext::Fall => "Fall",
    }
}

pub(crate) fn weighting_panel(
    ui: &mut egui::Ui,
    draft: &mut WeightingDraft,
    injuries: Option<&InjuryRegistry>,
    tables: Option<&InjuryTables>,
    last_save: &mut LastSaveRecord,
    root: Option<&Path>,
) {
    ui.heading("Weighting");
    ui.separator();

    category_combo(ui, draft, tables);
    source_combo(ui, draft, tables);

    let keys = sorted_keys(injuries);
    for severity in [Severity::Minor, Severity::Major, Severity::Critical] {
        bucket_rows(ui, draft, severity, &keys);
    }

    #[cfg(debug_assertions)]
    if let Some(root) = root {
        ui.separator();
        save_button(ui, draft, last_save, root);
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = last_save;
        let _ = root;
    }
}

fn category_combo(ui: &mut egui::Ui, draft: &mut WeightingDraft, tables: Option<&InjuryTables>) {
    ui.horizontal(|ui| {
        ui.label("Category table");
        let Some(tables) = tables else {
            ui.label("(loading…)");
            return;
        };
        let current = draft.category();
        let mut chosen: Option<InjuryCategory> = None;
        egui::ComboBox::from_id_salt("weighting_category_combo")
            .selected_text(category_label(current))
            .show_ui(ui, |ui| {
                for option in InjuryCategory::ALL {
                    if ui
                        .selectable_label(current == option, category_label(option))
                        .clicked()
                        && current != option
                    {
                        chosen = Some(option);
                    }
                }
            });
        if let Some(category) = chosen {
            draft.load_table(category, draft.context(), tables);
        }
    });
}

fn source_combo(ui: &mut egui::Ui, draft: &mut WeightingDraft, tables: Option<&InjuryTables>) {
    ui.horizontal(|ui| {
        ui.label("Wound source");
        let Some(tables) = tables else {
            ui.label("(loading…)");
            return;
        };
        let current = draft.context();
        let mut chosen: Option<DamageContext> = None;
        egui::ComboBox::from_id_salt("weighting_source_combo")
            .selected_text(context_label(current))
            .show_ui(ui, |ui| {
                for option in DamageContext::ALL {
                    if ui
                        .selectable_label(current == option, context_label(option))
                        .clicked()
                        && current != option
                    {
                        chosen = Some(option);
                    }
                }
            });
        if let Some(context) = chosen {
            draft.load_table(draft.category(), context, tables);
        }
    });
}

fn sorted_keys(injuries: Option<&InjuryRegistry>) -> Vec<InjuryName> {
    let mut keys: Vec<InjuryName> = injuries
        .map(|registry| registry.iter().map(|(key, _)| key.clone()).collect())
        .unwrap_or_default();
    keys.sort();
    keys
}

fn bucket_rows(
    ui: &mut egui::Ui,
    draft: &mut WeightingDraft,
    severity: Severity,
    keys: &[InjuryName],
) {
    ui.separator();
    ui.label(format!("{severity:?} bucket"));
    let weighting = draft.weighting_mut();
    let rows = match severity {
        Severity::Major => &mut weighting.major,
        Severity::Critical => &mut weighting.critical,
        _ => &mut weighting.minor,
    };
    let mut remove: Option<usize> = None;
    egui::Grid::new(("weighting_bucket", severity.rank())).show(ui, |ui| {
        for (index, row) in rows.iter_mut().enumerate() {
            injury_combo(ui, severity, index, &mut row.injury, keys);
            let mut weight: u32 = *row.weight;
            if ui.add(egui::DragValue::new(&mut weight)).changed() {
                row.weight = InjuryWeight::new(weight);
            }
            if ui.button("Remove").clicked() {
                remove = Some(index);
            }
            ui.end_row();
        }
    });
    if let Some(index) = remove {
        rows.remove(index);
    }
    let addable = !keys.is_empty();
    if ui
        .add_enabled(addable, egui::Button::new(format!("Add {severity:?} row")))
        .clicked()
        && let Some(first) = keys.first()
    {
        rows.push(WeightedInjuryEntry::new(
            first.clone(),
            InjuryWeight::new(1),
        ));
    }
}

fn injury_combo(
    ui: &mut egui::Ui,
    severity: Severity,
    index: usize,
    injury: &mut InjuryName,
    keys: &[InjuryName],
) {
    egui::ComboBox::from_id_salt(("weighting_row_injury", severity.rank(), index))
        .selected_text(injury.as_str().to_owned())
        .show_ui(ui, |ui| {
            for key in keys {
                let is_selected = injury == key;
                if ui.selectable_label(is_selected, key.as_str()).clicked() {
                    *injury = key.clone();
                }
            }
        });
}

#[cfg(debug_assertions)]
fn save_button(
    ui: &mut egui::Ui,
    draft: &WeightingDraft,
    last_save: &mut LastSaveRecord,
    root: &Path,
) {
    if !ui.button("Save weighting").clicked() {
        return;
    }
    let weighting = crate::injury_form::draft_to_weighting(draft);
    let written = crate::injury_form::write_weighting_in(root, &weighting);
    match &written {
        Ok(path) => bevy::log::info!(
            "weighting save: wrote `{}` / `{}` table to `{}`",
            category_label(weighting.category),
            context_label(weighting.context),
            path.display()
        ),
        Err(err) => bevy::log::error!(
            "weighting save: `{}` / `{}` table: {err}",
            category_label(weighting.category),
            context_label(weighting.context)
        ),
    }
    last_save.record(
        crate::EditorMode::Injury,
        crate::save_record::SaveOutcome::from_result(written),
    );
}
