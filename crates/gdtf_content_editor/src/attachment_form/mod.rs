//! The ATTACHMENT authoring mode's MODEL half (GTW-669) — the Workbench form that edits
//! an attachment item (`*.attachment.ron`: display name / the closed 6-slot mount / the
//! closed 13-effect list — the GTW-549/554 authoring schema) and saves it where the
//! GTW-619 [`AttachmentsFamily`](gdtf_content_families::AttachmentsFamily) folder loader
//! reads.
//!
//! Wiring-only module (module-layout rule 2). The working model (the state-scoped
//! [`AttachmentDraft`] resource and its pure mutators) lives in [`draft`]; the
//! loader-schema projection + the one-owner save path live in [`save`]. The egui FORM
//! that draws over this model is the shell's `egui_shell::attachment_form_ui` sibling
//! (the GTW-636 gang form split: model here, draw there). Follows the Gang / Armor /
//! Injury / Sprite modes at parity of pattern: load-any, create, edit fields (incl. the
//! effects list), save, load-back.

mod draft;
mod save;

pub use draft::AttachmentDraft;
pub use save::{attachment_file_name, attachment_save_path_in, draft_to_attachment_spec};
#[cfg(debug_assertions)]
pub use save::{write_attachment, write_attachment_in};

#[cfg(test)]
mod tests;
