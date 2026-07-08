//! The MELEE-WEAPON authoring mode's MODEL half (GTW-671) — the Workbench form that
//! edits a full [`MeleeWeaponSpec`](gdtf_battle_sim::weapon::MeleeWeaponSpec)
//! (`*.melee_weapon.ron`: the six SHARED damage-group fields, the melee-only `reach` +
//! `fight_mode` list, the `shove` tag, the slot declarations, and the attachment key
//! list) and saves it where the GTW-505/570
//! [`MeleeWeaponsFamily`](gdtf_content_families::MeleeWeaponsFamily) folder loader
//! reads.
//!
//! Wiring-only module (module-layout rule 2). The working model (the state-scoped
//! [`MeleeWeaponDraft`] resource and its pure mutators) lives in [`draft`]; the
//! loader-schema projection + the one-owner save path live in [`save`]. The egui FORM
//! that draws over this model is the shell's `egui_shell::melee_weapon_form_ui` sibling
//! (the GTW-636 gang form split: model here, draw there). Follows the GTW-670 ranged
//! WEAPON mode at parity of pattern: load-any, create, edit fields, save, load-back.

mod draft;
mod save;

pub use draft::MeleeWeaponDraft;
pub(crate) use draft::structural_swing_mode;
pub use save::{draft_to_melee_weapon_spec, melee_weapon_file_name, melee_weapon_save_path_in};
#[cfg(debug_assertions)]
pub use save::{write_melee_weapon, write_melee_weapon_in};

#[cfg(test)]
mod tests;
