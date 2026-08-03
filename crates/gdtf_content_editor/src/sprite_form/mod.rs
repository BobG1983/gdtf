//! Sprite authoring mode — draft and save.
mod draft;
mod save;

pub use draft::SpriteDraft;
pub use save::{draft_to_sprite_def, sprite_file_name, sprite_save_path_in};
#[cfg(debug_assertions)]
pub use save::{write_sprite, write_sprite_in};

#[cfg(test)]
mod tests;
