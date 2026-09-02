//! Terrain mode egui form.
mod blocking;
mod entry_sides;
mod fields;
mod leaves_behind;
mod on_death;
mod panel;
mod preview;
#[cfg(test)]
mod test;

pub(in crate::egui_shell) use fields::TerrainSaveContext;
pub(in crate::egui_shell) use panel::primary_panel;
pub(crate) use preview::ron_preview;
