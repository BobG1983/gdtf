//! Gang authoring mode — form draft and save for a roster edited outside the game binary.
mod draft;
mod save;

pub use draft::GangDraft;
pub use save::{draft_to_roster, gang_file_name, gang_save_path_in};
#[cfg(feature = "mcp")]
pub use save::{write_gang, write_gang_in};

#[cfg(test)]
mod tests;
