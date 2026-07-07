//! The ARMOR authoring mode's MODEL half (GTW-479) — the Workbench form that edits an
//! armor suit's six per-body-part pieces (`*.armor.ron`) and saves it where the GTW-269
//! armor folder loader reads.
//!
//! Wiring-only module (module-layout rule 2). The working model (the state-scoped
//! [`ArmorDraft`] resource) lives in [`draft`]; the loader-schema projection + the
//! one-owner save path live in [`save`]. The egui FORM that draws over this model is the
//! shell's `egui_shell::armor_form_ui` sibling (the GTW-636 gang form split: model here,
//! draw there). Follows the Gang mode at parity of pattern: load-any, create, edit
//! fields, save, load-back.

mod draft;
mod save;

pub use draft::ArmorDraft;
pub use save::{armor_file_name, armor_save_path_in, draft_to_spec};
#[cfg(debug_assertions)]
pub use save::{write_armor, write_armor_in};

#[cfg(test)]
mod tests;
