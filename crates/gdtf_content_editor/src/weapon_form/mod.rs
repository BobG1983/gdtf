//! The RANGED-WEAPON authoring mode's MODEL half (GTW-670) — the Workbench form that
//! edits a full [`WeaponSpec`](gdtf_battle_sim::weapon::WeaponSpec)
//! (`*.weapon.ron`: the seven ballistics/damage scalars, the damage-type / handedness /
//! trajectory vocabularies, the magazine, the fire-mode list, the `stable` / `shove`
//! tags, the slot declarations, the attachment key list, and the optional `dot` /
//! `on_death` records) and saves it where the GTW-257/570
//! [`WeaponsFamily`](gdtf_content_families::WeaponsFamily) folder loader reads.
//!
//! Wiring-only module (module-layout rule 2). The working model (the state-scoped
//! [`WeaponDraft`] resource and its pure mutators) lives in [`draft`]; the
//! loader-schema projection + the one-owner save path live in [`save`]. The egui FORM
//! that draws over this model is the shell's `egui_shell::weapon_form_ui` sibling (the
//! GTW-636 gang form split: model here, draw there). Follows the Gang / Armor / Injury
//! / Sprite / Attachment modes at parity of pattern: load-any, create, edit fields,
//! save, load-back.

mod draft;
mod save;

pub use draft::WeaponDraft;
pub(crate) use draft::{dot_turns_from_raw, structural_single_mode};
pub use save::{draft_to_weapon_spec, weapon_file_name, weapon_save_path_in};
#[cfg(debug_assertions)]
pub use save::{write_weapon, write_weapon_in};

#[cfg(test)]
mod tests;
