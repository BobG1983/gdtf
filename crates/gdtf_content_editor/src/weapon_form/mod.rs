//! Ranged weapon authoring mode — draft and save.
mod draft;
mod save;

pub use draft::WeaponDraft;
pub(crate) use draft::{dot_turns_from_raw, structural_single_mode};
pub use save::{draft_to_weapon_spec, weapon_file_name, weapon_save_path_in};
#[cfg(debug_assertions)]
pub use save::{write_weapon, write_weapon_in};

#[cfg(test)]
mod tests;
