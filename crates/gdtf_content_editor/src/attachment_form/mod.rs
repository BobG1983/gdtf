//! Attachment authoring mode — draft and save for the fixed effect list.
mod draft;
mod save;

pub use draft::AttachmentDraft;
pub use save::{attachment_file_name, attachment_save_path_in, draft_to_attachment_spec};
#[cfg(debug_assertions)]
pub use save::{write_attachment, write_attachment_in};

#[cfg(test)]
mod tests;
