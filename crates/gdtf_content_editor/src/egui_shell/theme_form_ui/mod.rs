//! Theme mode egui form.
mod autoload;
mod fields;
mod library;
mod stats;

#[cfg(test)]
mod tests;

pub(crate) use autoload::{load_theme_into_form, resolve_autoload};
pub(crate) use fields::field_stack;
pub(crate) use library::terrain_library_panel;
pub(crate) use stats::stats_panel;
