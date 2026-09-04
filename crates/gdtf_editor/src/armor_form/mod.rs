//! Armor authoring mode — form draft and save for an armor suit.
mod draft;
mod save;

pub use draft::ArmorDraft;
pub use save::{armor_file_name, armor_save_path_in, draft_to_spec};
#[cfg(feature = "mcp")]
pub use save::{write_armor, write_armor_in};

#[cfg(test)]
mod tests;
