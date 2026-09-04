//! Field authoring mode. Draft and save for one field catalog entry.
mod draft;
mod save;

pub use draft::FieldDraft;
pub use save::{draft_to_field, field_file_name, field_save_path_in};
#[cfg(feature = "mcp")]
pub use save::{write_field, write_field_in};

#[cfg(test)]
mod tests;
