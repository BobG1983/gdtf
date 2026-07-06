//! The THEME tab's LEFT resolved-stats readout panel (GTW-514 C3.1) — reuses
//! [`resolved_stats`] verbatim for the current default-floor terrain.

use bevy_egui::egui;
use gdtf_battle_sim::terrain::def::TerrainDefRegistry;

use crate::theme_form::{ThemeDraft, resolved_stats};

/// The placeholder text shown in the left stats panel when no default-floor terrain is selected
/// or the registry is absent.
const NO_FLOOR_STATS: &str = "Select a terrain\nas the default floor\nto see resolved stats.";

/// Draw the THEME-mode LEFT stats panel (C3.1) — the resolved-stats readout for the current
/// default-floor terrain. Reuses [`resolved_stats`] verbatim (C3.3): when the draft has a
/// default-floor key and the registry resolves it, shows the terrain's kind + HP + armor + band
/// summary and an [`egui::ProgressBar`] HP bar. When no floor is selected or the registry is
/// absent, shows a placeholder (never a panic).
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
