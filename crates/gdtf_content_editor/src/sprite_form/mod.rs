//! The SPRITE authoring mode's MODEL half (GTW-664) — the Workbench form that edits a
//! sprite definition (`*.spritedef.ron`: source / anchor / optional facings / optional
//! animation — the GTW-600 ruled schema) and saves it where the GTW-663
//! [`SpriteDefsFamily`](gdtf_content_families::sprites::SpriteDefsFamily) folder loader
//! reads.
//!
//! Wiring-only module (module-layout rule 2). The working model (the state-scoped
//! [`SpriteDraft`] resource and its pure mutators) lives in [`draft`]; the loader-schema
//! projection + the one-owner save path live in [`save`]. The egui FORM that draws over
//! this model is the shell's `egui_shell::sprite_form_ui` sibling (the GTW-636 gang form
//! split: model here, draw there). Follows the Gang / Armor / Injury modes at parity of
//! pattern: load-any, create, edit fields, save, load-back.

mod draft;
mod save;

pub use draft::SpriteDraft;
pub use save::{draft_to_sprite_def, sprite_file_name, sprite_save_path_in};
#[cfg(debug_assertions)]
pub use save::{write_sprite, write_sprite_in};

#[cfg(test)]
mod tests;
