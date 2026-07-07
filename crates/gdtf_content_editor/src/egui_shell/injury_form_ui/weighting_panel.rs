//! The INJURY tab's CENTRAL weighting SECTION (GTW-654 C2) — pick a context
//! table (one per [`InjuryCategory`]), then edit its three severity buckets as
//! `injury key → weight` rows. Row keys come from an injury-name [`ComboBox`]
//! sourced from the loaded [`InjuryRegistry`], so a DANGLING key is impossible to
//! author by construction (the combo offers only resolving keys, and Add-row is
//! disabled while the registry is empty).

use bevy_egui::egui;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{InjuryName, InjuryRegistry, InjuryTables, InjuryWeight, WeightedInjuryEntry},
    severity::Severity,
};

use crate::injury_form::WeightingDraft;

/// The display label for a context-table combo row — the category's authored RON
/// variant name (the def panel's `category_label` twin, kept local so each panel
/// file stays self-contained).
const fn category_label(category: InjuryCategory) -> &'static str {
    match category {
        InjuryCategory::Head => "Head",
        InjuryCategory::Torso => "Torso",
        InjuryCategory::Arm => "Arm",
        InjuryCategory::Leg => "Leg",
    }
}

/// Draw the INJURY-mode WEIGHTING section (GTW-654 C2): the context-table combo
/// (the four categories — choosing one loads its CURRENT built table through
/// [`WeightingDraft::load_category`]), the three per-severity bucket editors, and
/// the debug-only Save press. Falls back to a read-only "(loading…)" label while
/// the built tables are absent.
pub(crate) fn weighting_panel(
    ui: &mut egui::Ui,
    draft: &mut WeightingDraft,
    injuries: Option<&InjuryRegistry>,
    tables: Option<&InjuryTables>,
) {
    ui.heading("Weighting");
    ui.separator();

    context_combo(ui, draft, tables);

    // The sorted registry keys every row's injury combo offers (empty while the
    // registry is absent / holds nothing — Add-row is then disabled, so a key
    // outside the registry can never be authored).
    let keys = sorted_keys(injuries);
    for severity in [Severity::Minor, Severity::Major, Severity::Critical] {
        bucket_rows(ui, draft, severity, &keys);
    }

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_button(ui, draft);
    }
}

/// The context-table [`ComboBox`] — one row per [`InjuryCategory::ALL`] entry.
/// Choosing a DIFFERENT category re-loads the draft from that category's CURRENT
/// built table (an unpicked category's edits are the author's explicit discard —
/// the load-combo precedent); re-picking the current one is a no-op, so an open
/// combo never wipes edits. Disabled to a "(loading…)" label until the built
/// tables resolve.
fn context_combo(ui: &mut egui::Ui, draft: &mut WeightingDraft, tables: Option<&InjuryTables>) {
    ui.horizontal(|ui| {
        ui.label("Context table");
        let Some(tables) = tables else {
            ui.label("(loading…)");
            return;
        };
        let current = draft.category();
        let mut chosen: Option<InjuryCategory> = None;
        egui::ComboBox::from_id_salt("weighting_context_combo")
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
            draft.load_category(category, tables);
        }
    });
}

/// The sorted registry key list the per-row injury combos offer (empty when the
/// registry has not resolved / holds nothing).
fn sorted_keys(injuries: Option<&InjuryRegistry>) -> Vec<InjuryName> {
    let mut keys: Vec<InjuryName> = injuries
        .map(|registry| registry.iter().map(|(key, _)| key.clone()).collect())
        .unwrap_or_default();
    keys.sort();
    keys
}

/// One severity BUCKET's editor: a labelled grid of `injury combo | weight drag |
/// Remove` rows plus its Add-row button (disabled while no registry key exists —
/// a new row must name a resolving injury). A remove press is folded in AFTER the
/// loop (one structural edit per frame, the gang member-list precedent); a new
/// row seeds the FIRST sorted registry key at weight 1.
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
        // The tabled buckets are Minor/Major/Critical (None/Fatal are never
        // authored); this panel only iterates those three, so the fallthrough
        // arm is the Minor bucket.
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

/// One row's injury-key [`ComboBox`] — the sorted registry keys only (dangling
/// names impossible by construction). Salted by bucket + row index so every row
/// coexists.
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

/// The debug-only Save button — projects the draft through
/// [`draft_to_weighting`](crate::injury_form::draft_to_weighting) and writes it
/// via the one-owner [`write_weighting`](crate::injury_form::write_weighting)
/// path (`weighting/<category>.weighting.ron`). On a typed error it logs and
/// writes nothing (never a panic). Debug-only (the save-path precedent).
#[cfg(debug_assertions)]
fn save_button(ui: &mut egui::Ui, draft: &WeightingDraft) {
    if !ui.button("Save weighting").clicked() {
        return;
    }
    let weighting = crate::injury_form::draft_to_weighting(draft);
    match crate::injury_form::write_weighting(&weighting) {
        Ok(path) => bevy::log::info!(
            "weighting save: wrote `{}` table to `{}`",
            category_label(weighting.category),
            path.display()
        ),
        Err(err) => bevy::log::error!(
            "weighting save: `{}` table: {err}",
            category_label(weighting.category)
        ),
    }
}
