//! The ARMOR authoring mode's MODEL half (GTW-479) — the Workbench form that edits an
mod draft;
mod save;

pub use draft::ArmorDraft;
pub use save::{armor_file_name, armor_save_path_in, draft_to_spec};
#[cfg(debug_assertions)]
pub use save::{write_armor, write_armor_in};

#[cfg(test)]
mod tests;
