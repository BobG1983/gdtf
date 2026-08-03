//! The ATTACHMENT authoring mode's MODEL half (GTW-669) — the Workbench form that edits
//! closed 13-effect list — the GTW-549/554 authoring schema) and saves it where the
mod draft;
mod save;

pub use draft::AttachmentDraft;
pub use save::{attachment_file_name, attachment_save_path_in, draft_to_attachment_spec};
#[cfg(debug_assertions)]
pub use save::{write_attachment, write_attachment_in};

#[cfg(test)]
mod tests;
