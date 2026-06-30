//! The remaining minimally-stubbed per-mode forms (GTW-512 C1) — THEME / PREFAB.
//!
//! C1 stood up the SHELL only: the right panel shows the ACTIVE mode's form, and all three were
//! stubbed minimally — a labeled panel per mode — so NO mode looks inert in a screenshot (the
//! gate's Screenshot-QA phase reads the captured PNG). GTW-513 (C2) replaced the TERRAIN stub with
//! the real egui form ([`terrain_form_ui`](super::terrain_form_ui)); THEME (C3 / GTW-514) and
//! PREFAB + the texture viewport (C4 / GTW-515) are still stubbed here.
//!
//! Each stub takes the live [`egui::Ui`](bevy_egui::egui::Ui) of the right panel and the active
//! [`EditorMode`](crate::mode::EditorMode); the caller branches on the mode and calls exactly one.

use bevy_egui::egui;

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
