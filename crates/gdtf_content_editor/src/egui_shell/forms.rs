//! The three minimally-stubbed per-mode forms (GTW-512 C1) — TERRAIN / THEME / PREFAB.
//!
//! C1 is the SHELL only: the right [`SidePanel`](bevy_egui::egui::SidePanel) shows the ACTIVE
//! mode's form, and all three are stubbed minimally — a labeled panel per mode — so NO mode looks
//! inert in a screenshot (the gate's Screenshot-QA phase reads the captured PNG). The full forms
//! are the later children: TERRAIN is C2 (GTW-513), THEME is C3 (GTW-514), PREFAB + the texture
//! viewport is C4 (GTW-515).
//!
//! Each stub takes the live [`egui::Ui`](bevy_egui::egui::Ui) of the right panel and the active
//! [`EditorMode`](crate::mode::EditorMode); the caller branches on the mode and calls exactly one.

use bevy_egui::egui;

/// Draw the TERRAIN-mode form stub (GTW-512 C1; the full form is C2 / GTW-513).
pub(crate) fn terrain_form(ui: &mut egui::Ui) {
    ui.heading("Terrain");
    ui.separator();
    ui.label("Terrain definition authoring.");
    ui.label("Full form lands in C2 (GTW-513).");
}

/// Draw the THEME-mode form stub (GTW-512 C1; the full form is C3 / GTW-514).
pub(crate) fn theme_form(ui: &mut egui::Ui) {
    ui.heading("Theme");
    ui.separator();
    ui.label("Theme assembly (terrain references + default floor).");
    ui.label("Full form lands in C3 (GTW-514).");
}

/// Draw the PREFAB-mode form stub (GTW-512 C1; the full form + the size selector are C4 /
/// GTW-515).
pub(crate) fn prefab_form(ui: &mut egui::Ui) {
    ui.heading("Prefab");
    ui.separator();
    ui.label("Prefab painting (theme palette + grid size).");
    ui.label("Full form + viewport land in C4 (GTW-515).");
}
