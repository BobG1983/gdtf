//! Theme mode egui form.
mod autoload;
mod fields;
mod library;
mod stats;

#[cfg(test)]
mod tests;

pub(crate) use autoload::{load_theme_into_form, sync_theme_draft};
pub(in crate::egui_shell) use fields::field_stack;
pub(in crate::egui_shell) use library::terrain_library_panel;
pub(in crate::egui_shell) use stats::stats_panel;
