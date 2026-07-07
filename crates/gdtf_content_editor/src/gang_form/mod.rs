//! The GANG authoring mode's MODEL half (GTW-636) — the Workbench form that edits a
//! gang roster (`*.gang.ron`) and saves it where the GTW-415 gangs folder loader reads.
//!
//! Wiring-only module (module-layout rule 2). The working model (the state-scoped
//! [`GangDraft`] resource) lives in [`draft`]; the loader-schema projection + the
//! one-owner save path live in [`save`]. The egui FORM that draws over this model is the
//! shell's `egui_shell::gang_form_ui` sibling (the terrain / theme form split: model
//! here, draw there). The USER RULING (2026-07-06) behind this module: gangs are
//! authored OUTSIDE the game binary — this mode replaces the retired in-game gang
//! editor (`gdtf_app`'s `DebugGangEditor` scene) at full parity.

mod draft;
mod save;

pub use draft::GangDraft;
pub use save::{draft_to_roster, gang_file_name, gang_save_path_in};
#[cfg(debug_assertions)]
pub use save::{write_gang, write_gang_in};

#[cfg(test)]
mod tests;
