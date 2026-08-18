//! Terrain mode egui form.
mod blocking;
mod entry_sides;
mod fields;
mod on_death;
mod panel;
mod preview;

pub(in crate::egui_shell) use fields::TerrainSaveContext;
pub(in crate::egui_shell) use panel::primary_panel;
pub(crate) use preview::ron_preview;
