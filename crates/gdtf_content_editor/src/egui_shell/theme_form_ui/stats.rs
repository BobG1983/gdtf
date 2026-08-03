use bevy_egui::egui;
use gdtf_battle_sim::terrain::def::TerrainDefRegistry;

use crate::theme_form::{ThemeDraft, resolved_stats};

const NO_FLOOR_STATS: &str = "Select a terrain\nas the default floor\nto see resolved stats.";

pub(crate) fn stats_panel(
    ui: &mut egui::Ui,
    draft: &ThemeDraft,
    terrain: Option<&TerrainDefRegistry>,
) {
    ui.heading("Floor stats");
    ui.separator();
    let resolved = draft
        .default_floor()
        .zip(terrain)
        .and_then(|(key, reg)| reg.def(&key))
        .map(resolved_stats);
    if let Some((summary, fraction)) = resolved {
        ui.label(&summary);
        ui.separator();
        ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).text("HP"));
    } else {
        ui.label(NO_FLOOR_STATS);
    }
}
