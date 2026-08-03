//! The SPRITE authoring mode's MODEL half (GTW-664) — the Workbench form that edits a
mod draft;
mod save;

pub use draft::SpriteDraft;
pub use save::{draft_to_sprite_def, sprite_file_name, sprite_save_path_in};
#[cfg(debug_assertions)]
pub use save::{write_sprite, write_sprite_in};

#[cfg(test)]
mod tests;
