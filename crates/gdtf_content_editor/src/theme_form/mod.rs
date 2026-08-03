//! Theme authoring mode — model and save for a theme definition.
mod resolve;
mod save;
mod types;

#[cfg(test)]
mod tests;

pub(crate) use resolve::sim_kind_label;
pub use resolve::{floor_candidates, resolved_stats, slab_floor_candidates};
pub use save::{draft_to_theme_def, serialize_theme_def, validate_for_save};
#[cfg(debug_assertions)]
pub use save::{write_theme, write_theme_in};
pub use types::{SaveThemeError, ThemeDraft};
